use std::collections::BTreeSet;

use serde::Deserialize;

use super::{default_true, Context, Violation};
use crate::constants::keys;
use crate::document::node::get;
use crate::document::view::{Property, SchemaObject};

const UNNAMED: &str = "<unnamed>";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimaryKey {
    #[serde(default)]
    pub require_primary_key: bool,
    #[serde(default = "default_true")]
    pub require_position: bool,
}

pub fn check(options: &PrimaryKey, context: &Context<'_>) -> Vec<Violation> {
    context
        .contract
        .schema_objects()
        .iter()
        .flat_map(|object| check_object(options, context, object))
        .collect()
}

fn check_object(
    options: &PrimaryKey,
    context: &Context<'_>,
    object: &SchemaObject<'_>,
) -> Vec<Violation> {
    let owner = object.name().unwrap_or(UNNAMED);
    let properties = object
        .properties()
        .map(|list| list.properties())
        .unwrap_or_default();
    let keys: Vec<&Property<'_>> = properties
        .iter()
        .filter(|property| property.is_primary_key())
        .collect();

    let missing_key = (options.require_primary_key && keys.is_empty()).then(|| {
        Violation::new(
            format!("`{owner}` has no primary key"),
            context.location(object.node),
        )
    });
    let per_property = properties.iter().filter_map(|property| {
        let name = property.name().unwrap_or(UNNAMED);
        if property.is_primary_key()
            && options.require_position
            && property.primary_key_position().is_none()
        {
            Some(Violation::new(
                format!("primary key `{name}` has no primaryKeyPosition"),
                context.location(property.node),
            ))
        } else if !property.is_primary_key() && property.has_primary_key_position() {
            let node = get(property.node, keys::PRIMARY_KEY_POSITION).unwrap_or(property.node);
            Some(Violation::new(
                format!("`{name}` has a primaryKeyPosition but is not a primary key"),
                context.location(node),
            ))
        } else {
            None
        }
    });
    let positions: Vec<(i64, &Property<'_>)> = keys
        .iter()
        .filter_map(|property| {
            property
                .primary_key_position()
                .map(|position| (position, *property))
        })
        .collect();
    let duplicates = positions
        .iter()
        .enumerate()
        .filter(|(index, (position, _))| {
            positions[..*index]
                .iter()
                .any(|(earlier, _)| earlier == position)
        })
        .map(|(_, (position, property))| {
            Violation::new(
                format!("duplicate primaryKeyPosition {position} in `{owner}`"),
                context.location(property.node),
            )
        });
    let duplicates: Vec<Violation> = duplicates.collect();
    let distinct: BTreeSet<i64> = positions.iter().map(|(position, _)| *position).collect();
    let expected: BTreeSet<i64> = (1..).take(distinct.len()).collect();
    let gap = (duplicates.is_empty() && distinct != expected).then(|| {
        let found: Vec<String> = distinct.iter().map(ToString::to_string).collect();
        Violation::new(
            format!(
                "primaryKeyPosition values of `{owner}` should be 1..{} but are {}",
                distinct.len(),
                found.join(", ")
            ),
            context.location(object.node),
        )
    });

    missing_key
        .into_iter()
        .chain(per_property)
        .chain(duplicates)
        .chain(gap)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{assert_passes, messages, rule};

    fn contract(properties: &str) -> String {
        format!("schema:\n  - name: orders\n    properties:\n{properties}")
    }

    #[test]
    fn accepts_consistent_keys() {
        let kind = rule("type: primary_key\n");
        let source = contract(
            "      - name: a\n        primaryKey: true\n        primaryKeyPosition: 1\n      - name: b\n        primaryKey: true\n        primaryKeyPosition: 2\n      - name: c\n",
        );
        assert_passes(&kind, &source);
    }

    #[test]
    fn reports_key_without_position() {
        let kind = rule("type: primary_key\n");
        assert_eq!(
            messages(
                &kind,
                &contract("      - name: a\n        primaryKey: true\n")
            ),
            vec!["primary key `a` has no primaryKeyPosition"]
        );
    }

    #[test]
    fn position_can_be_optional() {
        let kind = rule("type: primary_key\nrequire_position: false\n");
        assert_passes(
            &kind,
            &contract("      - name: a\n        primaryKey: true\n"),
        );
    }

    #[test]
    fn reports_position_without_key() {
        let kind = rule("type: primary_key\n");
        assert_eq!(
            messages(
                &kind,
                &contract("      - name: a\n        primaryKeyPosition: 1\n")
            ),
            vec!["`a` has a primaryKeyPosition but is not a primary key"]
        );
    }

    #[test]
    fn default_position_is_not_reported() {
        let kind = rule("type: primary_key\n");
        assert_passes(
            &kind,
            &contract("      - name: a\n        primaryKeyPosition: -1\n"),
        );
    }

    #[test]
    fn reports_duplicate_positions() {
        let kind = rule("type: primary_key\n");
        let source = contract(
            "      - name: a\n        primaryKey: true\n        primaryKeyPosition: 1\n      - name: b\n        primaryKey: true\n        primaryKeyPosition: 1\n",
        );
        assert_eq!(
            messages(&kind, &source),
            vec!["duplicate primaryKeyPosition 1 in `orders`"]
        );
    }

    #[test]
    fn reports_gaps_in_positions() {
        let kind = rule("type: primary_key\n");
        let source = contract(
            "      - name: a\n        primaryKey: true\n        primaryKeyPosition: 1\n      - name: b\n        primaryKey: true\n        primaryKeyPosition: 3\n",
        );
        assert_eq!(
            messages(&kind, &source),
            vec!["primaryKeyPosition values of `orders` should be 1..2 but are 1, 3"]
        );
    }

    #[test]
    fn reports_missing_primary_key_when_required() {
        let kind = rule("type: primary_key\nrequire_primary_key: true\n");
        assert_eq!(
            messages(&kind, &contract("      - name: a\n")),
            vec!["`orders` has no primary key"]
        );
    }
}
