use serde::Deserialize;

use super::{Context, Violation};
use crate::constants::keys;
use crate::document::node::{as_str, get};
use crate::document::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameScope {
    Schema,
    Property,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueNames {
    pub scope: NameScope,
    #[serde(default)]
    pub case_sensitive: bool,
}

pub fn check(options: &UniqueNames, context: &Context<'_>) -> Vec<Violation> {
    let contract = context.contract;
    let groups: Vec<Vec<&Node<'_>>> = match options.scope {
        NameScope::Schema => vec![contract
            .schema_objects()
            .iter()
            .map(|object| object.node)
            .collect()],
        NameScope::Property => contract
            .property_lists(true)
            .iter()
            .map(|list| {
                list.properties()
                    .iter()
                    .map(|property| property.node)
                    .collect()
            })
            .collect(),
    };
    let kind = match options.scope {
        NameScope::Schema => "schema",
        NameScope::Property => "property",
    };
    groups
        .iter()
        .flat_map(|group| {
            let names: Vec<(&Node<'_>, String)> = group
                .iter()
                .filter_map(|owner| {
                    let node = get(owner, keys::NAME)?;
                    let name = as_str(node)?;
                    let key = if options.case_sensitive {
                        name.to_string()
                    } else {
                        name.to_lowercase()
                    };
                    Some((node, key))
                })
                .collect();
            names
                .iter()
                .enumerate()
                .filter(|(index, (_, key))| {
                    names[..*index].iter().any(|(_, earlier)| earlier == key)
                })
                .map(|(_, (node, _))| {
                    Violation::new(
                        format!(
                            "duplicate {kind} name `{}`",
                            as_str(node).unwrap_or_default()
                        ),
                        context.location(node),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{messages, rule};

    #[test]
    fn reports_duplicate_property_names_case_insensitively() {
        let kind = rule("type: unique_names\nscope: property\n");
        let contract =
            "schema:\n  - name: t\n    properties:\n      - name: id\n      - name: ID\n";
        assert_eq!(
            messages(&kind, contract),
            vec!["duplicate property name `ID`"]
        );
    }

    #[test]
    fn case_sensitive_mode() {
        let kind = rule("type: unique_names\nscope: property\ncase_sensitive: true\n");
        let contract =
            "schema:\n  - name: t\n    properties:\n      - name: id\n      - name: ID\n";
        assert!(messages(&kind, contract).is_empty());
    }

    #[test]
    fn same_name_in_different_lists_is_fine() {
        let kind = rule("type: unique_names\nscope: property\n");
        let contract = "schema:\n  - name: a\n    properties:\n      - name: id\n  - name: b\n    properties:\n      - name: id\n";
        assert!(messages(&kind, contract).is_empty());
    }

    #[test]
    fn reports_duplicate_schema_names() {
        let kind = rule("type: unique_names\nscope: schema\n");
        assert_eq!(
            messages(&kind, "schema:\n  - name: a\n  - name: a\n"),
            vec!["duplicate schema name `a`"]
        );
    }
}
