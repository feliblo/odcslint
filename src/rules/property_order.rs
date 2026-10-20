use serde::Deserialize;

use super::{default_true, Context, Violation};
use crate::config::pattern::Pattern;
use crate::document::view::{Property, PropertyList};
use crate::fix::block_seq::reorder;
use crate::fix::{Fix, Permutation};

const DEFAULT_METADATA_PREFIX: &str = "_";
const UNNAMED: &str = "<unnamed>";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyOrder {
    #[serde(default = "default_true")]
    pub primary_keys_first: bool,
    #[serde(default)]
    pub metadata_columns: MetadataColumns,
    #[serde(default = "default_true")]
    pub recursive: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MetadataColumns {
    pub prefixes: Vec<String>,
    pub suffixes: Vec<String>,
    pub names: Vec<String>,
    pub pattern: Option<Pattern>,
}

impl Default for MetadataColumns {
    fn default() -> Self {
        Self {
            prefixes: vec![DEFAULT_METADATA_PREFIX.to_string()],
            suffixes: Vec::new(),
            names: Vec::new(),
            pattern: None,
        }
    }
}

impl MetadataColumns {
    fn matches(&self, name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix.as_str()))
            || self
                .suffixes
                .iter()
                .any(|suffix| name.ends_with(suffix.as_str()))
            || self.names.iter().any(|candidate| candidate == name)
            || self
                .pattern
                .as_ref()
                .is_some_and(|pattern| pattern.is_match(name))
    }
}

fn rank(options: &PropertyOrder, property: Property<'_>) -> (u8, i64) {
    if options.primary_keys_first && property.is_primary_key() {
        (0, property.primary_key_position().unwrap_or(i64::MAX))
    } else if property
        .name()
        .is_some_and(|name| options.metadata_columns.matches(name))
    {
        (2, 0)
    } else {
        (1, 0)
    }
}

/// Stable sort, so ties keep their current order.
pub fn expected_order(options: &PropertyOrder, properties: &[Property<'_>]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..properties.len()).collect();
    order.sort_by_key(|&index| rank(options, properties[index]));
    order
}

pub fn check(options: &PropertyOrder, context: &Context<'_>) -> Vec<Violation> {
    context
        .contract
        .property_lists(options.recursive)
        .iter()
        .filter_map(|list| check_list(options, context, list))
        .collect()
}

fn check_list(
    options: &PropertyOrder,
    context: &Context<'_>,
    list: &PropertyList<'_>,
) -> Option<Violation> {
    let properties = list.properties();
    let order = expected_order(options, &properties);
    let position = order
        .iter()
        .enumerate()
        .position(|(position, &index)| position != index)?;
    let name_of = |index: usize| properties[index].name().unwrap_or(UNNAMED);
    let message = format!(
        "expected `{}` at position {} of `{}`, found `{}`",
        name_of(order[position]),
        position + 1,
        list.owner.unwrap_or(UNNAMED),
        name_of(position),
    );
    let fix = reorder(context.document, list.node, &order).map(|edit| Fix {
        edit,
        permutation: Permutation {
            pointer: list.pointer.clone(),
            order,
        },
    });
    Some(Violation {
        message,
        location: context.location(properties[position].node),
        fix,
    })
}

#[cfg(test)]
mod tests {
    use crate::diagnostic::Location;
    use crate::rules::test_support::{messages, rule, violations_at};

    fn contract(properties: &str) -> String {
        format!("schema:\n  - name: orders\n    properties:\n{properties}")
    }

    const ORDERED: &str = "      - name: order_id\n        primaryKey: true\n        primaryKeyPosition: 1\n      - name: amount\n      - name: _loaded_at\n";

    #[test]
    fn accepts_expected_order() {
        let kind = rule("type: property_order\n");
        assert!(messages(&kind, &contract(ORDERED)).is_empty());
    }

    #[test]
    fn reports_metadata_column_before_regular_one() {
        let kind = rule("type: property_order\n");
        let source = contract("      - name: _loaded_at\n      - name: amount\n");
        let violations = violations_at(&kind, &source, "c.odcs.yaml");
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].message,
            "expected `amount` at position 1 of `orders`, found `_loaded_at`"
        );
        assert_eq!(violations[0].location, Location { line: 4, column: 9 });
        assert!(violations[0].fix.is_some());
    }

    #[test]
    fn orders_primary_keys_by_position() {
        let kind = rule("type: property_order\n");
        let source = contract(
            "      - name: b\n        primaryKey: true\n        primaryKeyPosition: 2\n      - name: a\n        primaryKey: true\n        primaryKeyPosition: 1\n",
        );
        let violations = violations_at(&kind, &source, "c.odcs.yaml");
        assert_eq!(
            violations[0].fix.as_ref().unwrap().permutation.order,
            vec![1, 0]
        );
    }

    #[test]
    fn primary_key_wins_over_metadata_prefix() {
        let kind = rule("type: property_order\n");
        let source =
            contract("      - name: _id\n        primaryKey: true\n      - name: amount\n");
        assert!(messages(&kind, &source).is_empty());
    }

    #[test]
    fn primary_keys_first_can_be_disabled() {
        let kind = rule("type: property_order\nprimary_keys_first: false\n");
        let source = contract("      - name: amount\n      - name: id\n        primaryKey: true\n");
        assert!(messages(&kind, &source).is_empty());
    }

    #[test]
    fn custom_metadata_columns() {
        let kind = rule("type: property_order\nmetadata_columns:\n  prefixes: []\n  suffixes: [_ts]\n  names: [etl_batch]\n  pattern: '^dw_'\n");
        let source = contract("      - name: etl_batch\n      - name: load_ts\n      - name: dw_hash\n      - name: _keep\n");
        let violations = violations_at(&kind, &source, "c.odcs.yaml");
        assert_eq!(
            violations[0].fix.as_ref().unwrap().permutation.order,
            vec![3, 0, 1, 2]
        );
    }

    #[test]
    fn checks_nested_lists_when_recursive() {
        let kind = rule("type: property_order\n");
        let source = contract(
            "      - name: address\n        logicalType: object\n        properties:\n          - name: _raw\n          - name: street\n",
        );
        assert_eq!(messages(&kind, &source).len(), 1);
        let kind = rule("type: property_order\nrecursive: false\n");
        assert!(messages(&kind, &source).is_empty());
    }

    #[test]
    fn flow_sequences_are_reported_without_fix() {
        let kind = rule("type: property_order\n");
        let source = "schema:\n  - name: t\n    properties: [{name: _a}, {name: b}]\n";
        let violations = violations_at(&kind, source, "c.odcs.yaml");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].fix.is_none());
    }
}
