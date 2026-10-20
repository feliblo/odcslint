use serde::Deserialize;

use super::{Context, Violation};
use crate::constants::keys;
use crate::document::node::{as_seq, entry};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeprecatedFields {}

const DEPRECATED_KEYS: [(&str, &str); 2] = [
    (
        keys::DATA_PRODUCT,
        "describe the data product outside the contract",
    ),
    (
        keys::SLA_DEFAULT_ELEMENT,
        "set `element` on each slaProperties entry",
    ),
];

pub fn check(_options: &DeprecatedFields, context: &Context<'_>) -> Vec<Violation> {
    let root = context.contract.root();
    let deprecated_keys = DEPRECATED_KEYS.iter().filter_map(|(key, hint)| {
        entry(root, key).map(|(key_node, _)| {
            Violation::new(
                format!("`{key}` is deprecated; {hint}"),
                context.location(key_node),
            )
        })
    });
    let team_list = entry(root, keys::TEAM)
        .filter(|(_, value)| as_seq(value).is_some())
        .map(|(key_node, _)| {
            Violation::new(
                "`team` as a list is deprecated; use `team.members`".to_string(),
                context.location(key_node),
            )
        });
    deprecated_keys.chain(team_list).collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{assert_passes, messages, rule};

    #[test]
    fn reports_deprecated_keys() {
        let kind = rule("type: deprecated_fields\n");
        assert_eq!(
            messages(&kind, "dataProduct: x\nslaDefaultElement: y\n"),
            vec![
                "`dataProduct` is deprecated; describe the data product outside the contract",
                "`slaDefaultElement` is deprecated; set `element` on each slaProperties entry"
            ]
        );
    }

    #[test]
    fn reports_team_list() {
        let kind = rule("type: deprecated_fields\n");
        assert_eq!(
            messages(&kind, "team:\n  - username: a\n"),
            vec!["`team` as a list is deprecated; use `team.members`"]
        );
    }

    #[test]
    fn accepts_team_object() {
        let kind = rule("type: deprecated_fields\n");
        assert_passes(&kind, "team:\n  members:\n    - username: a\n");
    }
}
