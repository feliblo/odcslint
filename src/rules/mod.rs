pub mod allowed_values;
pub mod deprecated_fields;
pub mod file_naming;
pub mod has_description;
pub mod id_format;
pub mod naming_convention;
pub mod primary_key;
pub mod property_order;
pub mod required_fields;
pub mod required_tags;
pub mod unique_names;

use serde::Deserialize;
use strum::{AsRefStr, EnumDiscriminants, EnumIter, IntoStaticStr};

use crate::config::Rule;
use crate::diagnostic::{Finding, Location};
use crate::document::view::Contract;
use crate::document::{Document, Node};
use crate::fix::Fix;

#[derive(Debug, Deserialize, AsRefStr, EnumDiscriminants)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(name(RuleType), derive(EnumIter, IntoStaticStr))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[remain::sorted]
pub enum RuleKind {
    AllowedValues(allowed_values::AllowedValues),
    DeprecatedFields(deprecated_fields::DeprecatedFields),
    FileNaming(file_naming::FileNaming),
    HasDescription(has_description::HasDescription),
    IdFormat(id_format::IdFormat),
    NamingConvention(naming_convention::NamingConvention),
    PrimaryKey(primary_key::PrimaryKey),
    PropertyOrder(property_order::PropertyOrder),
    RequiredFields(required_fields::RequiredFields),
    RequiredTags(required_tags::RequiredTags),
    UniqueNames(unique_names::UniqueNames),
}

impl RuleKind {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::AllowedValues(options) => options.validate(),
            Self::FileNaming(options) => options.validate(),
            Self::RequiredFields(options) => options.validate(),
            Self::RequiredTags(options) => options.validate(),
            Self::DeprecatedFields(_)
            | Self::HasDescription(_)
            | Self::IdFormat(_)
            | Self::NamingConvention(_)
            | Self::PrimaryKey(_)
            | Self::PropertyOrder(_)
            | Self::UniqueNames(_) => Ok(()),
        }
    }
}

impl RuleType {
    pub fn name(self) -> &'static str {
        self.into()
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::AllowedValues => {
                "A field (owners, domain, status, types, tags, ...) only uses allowed values"
            }
            Self::DeprecatedFields => "No deprecated ODCS fields are used",
            Self::FileNaming => "File names follow a pattern and/or match the contract id or name",
            Self::HasDescription => "Contracts, schema objects or properties have a description",
            Self::IdFormat => "The contract id is a UUID or matches a pattern",
            Self::NamingConvention => "Schema object and property names follow a naming convention",
            Self::PrimaryKey => "primaryKey and primaryKeyPosition are consistent",
            Self::PropertyOrder => "Primary keys come first and metadata columns last",
            Self::RequiredFields => "Fields are present and non-empty",
            Self::RequiredTags => "Tags are present",
            Self::UniqueNames => "Schema object or property names are unique",
        }
    }

    pub const fn fixable(self) -> bool {
        matches!(self, Self::PropertyOrder)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub document: &'a Document<'a>,
    pub contract: Contract<'a>,
    pub path: &'a str,
}

impl<'a> Context<'a> {
    pub const fn new(document: &'a Document<'a>, path: &'a str) -> Self {
        Self {
            document,
            contract: Contract::new(document.root()),
            path,
        }
    }

    pub fn location(&self, node: &Node<'_>) -> Location {
        self.document.location(node)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub message: String,
    pub location: Location,
    pub fix: Option<Fix>,
}

impl Violation {
    pub const fn new(message: String, location: Location) -> Self {
        Self {
            message,
            location,
            fix: None,
        }
    }
}

pub fn run(rules: &[Rule], context: &Context<'_>) -> Vec<(Finding, Option<Fix>)> {
    rules
        .iter()
        .filter(|rule| rule.applies_to(context.path))
        .flat_map(|rule| {
            check(&rule.kind, context)
                .into_iter()
                .map(move |violation| {
                    let finding = Finding {
                        rule: rule.kind.as_ref().to_string(),
                        name: rule.display_name().to_string(),
                        severity: rule.severity,
                        message: violation.message,
                        location: violation.location,
                        fixable: violation.fix.is_some(),
                    };
                    (finding, violation.fix)
                })
        })
        .collect()
}

fn check(kind: &RuleKind, context: &Context<'_>) -> Vec<Violation> {
    match kind {
        RuleKind::AllowedValues(options) => allowed_values::check(options, context),
        RuleKind::DeprecatedFields(options) => deprecated_fields::check(options, context),
        RuleKind::FileNaming(options) => file_naming::check(options, context),
        RuleKind::HasDescription(options) => has_description::check(options, context),
        RuleKind::IdFormat(options) => id_format::check(options, context),
        RuleKind::NamingConvention(options) => naming_convention::check(options, context),
        RuleKind::PrimaryKey(options) => primary_key::check(options, context),
        RuleKind::PropertyOrder(options) => property_order::check(options, context),
        RuleKind::RequiredFields(options) => required_fields::check(options, context),
        RuleKind::RequiredTags(options) => required_tags::check(options, context),
        RuleKind::UniqueNames(options) => unique_names::check(options, context),
    }
}

pub fn describe(kind: &str, node: &Node<'_>) -> String {
    crate::document::node::get(node, crate::constants::keys::NAME)
        .and_then(crate::document::node::as_str)
        .map_or_else(|| kind.to_string(), |name| format!("{kind} `{name}`"))
}

pub fn list(values: &[impl AsRef<str>]) -> String {
    const SHOWN: usize = 5;
    let shown: Vec<&str> = values.iter().take(SHOWN).map(AsRef::as_ref).collect();
    if values.len() > SHOWN {
        format!("{}, ... ({} total)", shown.join(", "), values.len())
    } else {
        shown.join(", ")
    }
}

pub const fn default_true() -> bool {
    true
}

#[cfg(test)]
pub mod test_support {

    use super::{check, Context, RuleKind, Violation};
    use crate::document::Document;

    pub fn parse_rule(yaml: &str) -> RuleKind {
        let document = Document::parse(yaml).unwrap();
        serde_json::from_value(crate::document::to_json::to_json(document.root())).unwrap()
    }

    pub fn rule(yaml: &str) -> RuleKind {
        let kind = parse_rule(yaml);
        kind.validate().unwrap();
        kind
    }

    pub fn violations_at(kind: &RuleKind, contract: &str, path: &str) -> Vec<Violation> {
        let document = Document::parse(contract).unwrap();
        check(kind, &Context::new(&document, path))
    }

    pub fn assert_passes(kind: &RuleKind, contract: &str) {
        assert_eq!(messages(kind, contract), Vec::<String>::new());
    }

    pub fn messages(kind: &RuleKind, contract: &str) -> Vec<String> {
        violations_at(kind, contract, "contract.odcs.yaml")
            .into_iter()
            .map(|violation| violation.message)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn list_truncates_long_value_lists() {
        let values: Vec<String> = (1..=7).map(|n| n.to_string()).collect();
        assert_eq!(list(&values), "1, 2, 3, 4, 5, ... (7 total)");
    }

    #[test]
    fn rule_types_use_snake_case_names() {
        let names: Vec<&str> = RuleType::iter().map(RuleType::name).collect();
        assert!(names.contains(&"property_order"));
        assert!(names
            .iter()
            .all(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_')));
    }

    #[test]
    fn only_property_order_is_fixable() {
        let fixable: Vec<RuleType> = RuleType::iter().filter(|kind| kind.fixable()).collect();
        assert_eq!(fixable, vec![RuleType::PropertyOrder]);
    }
}
