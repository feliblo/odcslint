use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use super::{Context, Violation};
use crate::config::pattern::Pattern;
use crate::constants::keys;
use crate::document::node::scalar_text;

const UUID_PATTERN: &str =
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$";

static UUID: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(UUID_PATTERN).ok());

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdFormat {
    pub pattern: Option<Pattern>,
}

pub fn check(options: &IdFormat, context: &Context<'_>) -> Vec<Violation> {
    let Some(node) = context.contract.field(keys::ID) else {
        return Vec::new();
    };
    let id = scalar_text(node).unwrap_or_default();
    let (valid, problem) = options.pattern.as_ref().map_or_else(
        || {
            let valid = UUID.as_ref().is_some_and(|uuid| uuid.is_match(&id));
            (valid, "is not a UUID".to_string())
        },
        |pattern| {
            let problem = format!("does not match pattern `{}`", pattern.as_str());
            (pattern.is_match(&id), problem)
        },
    );
    (!valid)
        .then(|| {
            Violation::new(
                format!("contract id `{id}` {problem}"),
                context.location(node),
            )
        })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{messages, rule};

    #[test]
    fn accepts_uuid() {
        let kind = rule("type: id_format\n");
        assert!(messages(&kind, "id: 53581432-6c55-4ba2-a65f-72344a91553c\n").is_empty());
    }

    #[test]
    fn rejects_non_uuid() {
        let kind = rule("type: id_format\n");
        assert_eq!(
            messages(&kind, "id: orders\n"),
            vec!["contract id `orders` is not a UUID"]
        );
    }

    #[test]
    fn custom_pattern() {
        let kind = rule("type: id_format\npattern: '^urn:'\n");
        assert_eq!(
            messages(&kind, "id: orders\n"),
            vec!["contract id `orders` does not match pattern `^urn:`"]
        );
    }

    #[test]
    fn missing_id_is_left_to_the_schema() {
        let kind = rule("type: id_format\n");
        assert!(messages(&kind, "name: x\n").is_empty());
    }
}
