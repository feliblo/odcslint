use std::borrow::Cow;

use serde::Deserialize;

use super::{describe, list, Context, Violation};
use crate::constants::keys;
use crate::document::node::{get, scalar_items, scalar_text};
use crate::document::view::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Criteria {
    #[default]
    All,
    Any,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredTags {
    #[serde(default = "default_scope")]
    pub scope: Scope,
    pub tags: Vec<String>,
    #[serde(default)]
    pub criteria: Criteria,
}

const fn default_scope() -> Scope {
    Scope::Contract
}

impl RequiredTags {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.tags.is_empty() {
            Err("`tags` must not be empty".to_string())
        } else {
            Ok(())
        }
    }
}

pub fn check(options: &RequiredTags, context: &Context<'_>) -> Vec<Violation> {
    context
        .contract
        .nodes_in(options.scope)
        .into_iter()
        .filter_map(|node| {
            let present: Vec<String> = get(node, keys::TAGS)
                .map(scalar_items)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|tag| scalar_text(tag).map(Cow::into_owned))
                .collect();
            let missing: Vec<&String> = options
                .tags
                .iter()
                .filter(|tag| !present.contains(tag))
                .collect();
            let subject = match options.scope {
                Scope::Contract => "contract".to_string(),
                Scope::Schema => describe("schema", node),
                Scope::Property => describe("property", node),
            };
            let message = match options.criteria {
                Criteria::All if !missing.is_empty() => Some(format!(
                    "{subject} is missing required tags: {}",
                    list(&missing)
                )),
                Criteria::Any if missing.len() == options.tags.len() => Some(format!(
                    "{subject} needs at least one of the tags: {}",
                    list(&options.tags)
                )),
                Criteria::All | Criteria::Any => None,
            };
            message.map(|message| Violation::new(message, context.location(node)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{messages, rule};

    #[test]
    fn all_tags_required() {
        let kind = rule("type: required_tags\ntags: [pii-reviewed, owner-approved]\n");
        assert_eq!(
            messages(&kind, "tags: [pii-reviewed]\n"),
            vec!["contract is missing required tags: owner-approved"]
        );
    }

    #[test]
    fn any_tag_suffices() {
        let kind = rule("type: required_tags\ntags: [gold, silver]\ncriteria: any\n");
        assert!(messages(&kind, "tags: [silver]\n").is_empty());
        assert_eq!(
            messages(&kind, "tags: [bronze]\n"),
            vec!["contract needs at least one of the tags: gold, silver"]
        );
    }

    #[test]
    fn schema_scope() {
        let kind = rule("type: required_tags\nscope: schema\ntags: [core]\n");
        assert_eq!(
            messages(
                &kind,
                "schema:\n  - name: t\n    tags: [core]\n  - name: u\n"
            ),
            vec!["schema `u` is missing required tags: core"]
        );
    }
}
