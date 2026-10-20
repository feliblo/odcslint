use serde::Deserialize;

use super::{describe, Context, Violation};
use crate::document::node::{get_path, is_empty};
use crate::document::view::Scope;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredFields {
    #[serde(default = "default_scope")]
    pub scope: Scope,
    pub fields: Vec<String>,
}

const fn default_scope() -> Scope {
    Scope::Contract
}

impl RequiredFields {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.fields.is_empty() {
            Err("`fields` must not be empty".to_string())
        } else {
            Ok(())
        }
    }
}

pub fn check(options: &RequiredFields, context: &Context<'_>) -> Vec<Violation> {
    context
        .contract
        .nodes_in(options.scope)
        .into_iter()
        .flat_map(|node| {
            options
                .fields
                .iter()
                .filter(move |field| get_path(node, field).is_none_or(is_empty))
                .map(move |field| {
                    let message = match options.scope {
                        Scope::Contract => format!("missing required field `{field}`"),
                        Scope::Schema => format!(
                            "{} is missing required field `{field}`",
                            describe("schema", node)
                        ),
                        Scope::Property => {
                            format!(
                                "{} is missing required field `{field}`",
                                describe("property", node)
                            )
                        }
                    };
                    Violation::new(message, context.location(node))
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{messages, rule};

    #[test]
    fn accepts_present_fields() {
        let kind = rule("type: required_fields\nfields: [domain, description.purpose]\n");
        assert!(messages(&kind, "domain: sales\ndescription:\n  purpose: Orders\n").is_empty());
    }

    #[test]
    fn reports_missing_and_empty_fields() {
        let kind = rule("type: required_fields\nfields: [domain, description.purpose]\n");
        assert_eq!(
            messages(&kind, "domain: ''\n"),
            vec![
                "missing required field `domain`",
                "missing required field `description.purpose`"
            ]
        );
    }

    #[test]
    fn checks_every_property() {
        let kind = rule("type: required_fields\nscope: property\nfields: [logicalType]\n");
        let contract = "schema:\n  - name: t\n    properties:\n      - name: a\n        logicalType: string\n      - name: b\n";
        assert_eq!(
            messages(&kind, contract),
            vec!["property `b` is missing required field `logicalType`"]
        );
    }

    #[test]
    fn checks_schema_objects() {
        let kind = rule("type: required_fields\nscope: schema\nfields: [physicalName]\n");
        assert_eq!(
            messages(&kind, "schema:\n  - name: t\n"),
            vec!["schema `t` is missing required field `physicalName`"]
        );
    }
}
