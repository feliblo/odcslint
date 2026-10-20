use regex::Regex;
use serde::Deserialize;

use super::{Context, Violation};
use crate::constants::keys;
use crate::document::node::{as_str, get};
use crate::document::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameTarget {
    #[serde(alias = "schema.name")]
    SchemaName,
    #[serde(alias = "schema.physicalName")]
    SchemaPhysicalName,
    #[serde(alias = "schema.properties.name")]
    PropertyName,
    #[serde(alias = "schema.properties.physicalName")]
    PropertyPhysicalName,
}

impl NameTarget {
    const fn label(self) -> &'static str {
        match self {
            Self::SchemaName => "schema name",
            Self::SchemaPhysicalName => "schema physicalName",
            Self::PropertyName => "property name",
            Self::PropertyPhysicalName => "property physicalName",
        }
    }
}

/// Leading underscores are allowed in the snake case variants so metadata columns like `_loaded_at` pass.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
#[expect(
    clippy::enum_variant_names,
    reason = "variants are named after the conventions they describe"
)]
enum NamedConvention {
    CamelCase,
    KebabCase,
    PascalCase,
    ScreamingSnakeCase,
    SnakeCase,
}

impl NamedConvention {
    const fn label(self) -> &'static str {
        match self {
            Self::CamelCase => "camel_case",
            Self::KebabCase => "kebab_case",
            Self::PascalCase => "pascal_case",
            Self::ScreamingSnakeCase => "screaming_snake_case",
            Self::SnakeCase => "snake_case",
        }
    }

    const fn regex(self) -> &'static str {
        match self {
            Self::CamelCase => r"^[a-z][a-zA-Z0-9]*$",
            Self::KebabCase => r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$",
            Self::PascalCase => r"^[A-Z][a-zA-Z0-9]*$",
            Self::ScreamingSnakeCase => r"^_*[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$",
            Self::SnakeCase => r"^_*[a-z][a-z0-9]*(_[a-z0-9]+)*$",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConventionSpec {
    Named(NamedConvention),
    Custom { pattern: String },
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "ConventionSpec")]
pub struct Convention {
    label: String,
    regex: Regex,
}

impl TryFrom<ConventionSpec> for Convention {
    type Error = String;

    fn try_from(spec: ConventionSpec) -> Result<Self, Self::Error> {
        let (label, pattern) = match spec {
            ConventionSpec::Named(named) => (named.label().to_string(), named.regex().to_string()),
            ConventionSpec::Custom { pattern } => (format!("pattern `{pattern}`"), pattern),
        };
        Regex::new(&pattern)
            .map(|regex| Self { label, regex })
            .map_err(|error| format!("invalid regex `{pattern}`: {error}"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamingConvention {
    pub target: NameTarget,
    pub convention: Convention,
}

pub fn check(options: &NamingConvention, context: &Context<'_>) -> Vec<Violation> {
    let contract = context.contract;
    let owners: Vec<&Node<'_>> = match options.target {
        NameTarget::SchemaName | NameTarget::SchemaPhysicalName => contract
            .schema_objects()
            .iter()
            .map(|object| object.node)
            .collect(),
        NameTarget::PropertyName | NameTarget::PropertyPhysicalName => contract
            .properties()
            .iter()
            .map(|property| property.node)
            .collect(),
    };
    let key = match options.target {
        NameTarget::SchemaName | NameTarget::PropertyName => keys::NAME,
        NameTarget::SchemaPhysicalName | NameTarget::PropertyPhysicalName => keys::PHYSICAL_NAME,
    };
    owners
        .into_iter()
        .filter_map(|owner| get(owner, key))
        .filter_map(|node| {
            let name = as_str(node)?;
            (!options.convention.regex.is_match(name)).then(|| {
                Violation::new(
                    format!(
                        "{} `{name}` does not follow {}",
                        options.target.label(),
                        options.convention.label
                    ),
                    context.location(node),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{messages, rule};

    const CONTRACT: &str = "\
schema:
  - name: Orders
    physicalName: orders_v1
    properties:
      - name: order_id
      - name: _loaded_at
      - name: customerName
";

    #[test]
    fn snake_case_properties_allow_leading_underscore() {
        let kind = rule("type: naming_convention\ntarget: property_name\nconvention: snake_case\n");
        assert_eq!(
            messages(&kind, CONTRACT),
            vec!["property name `customerName` does not follow snake_case"]
        );
    }

    #[test]
    fn accepts_odcs_path_as_target() {
        let kind = rule(
            "type: naming_convention\ntarget: schema.properties.name\nconvention: snake_case\n",
        );
        assert_eq!(
            messages(&kind, CONTRACT),
            vec!["property name `customerName` does not follow snake_case"]
        );
    }

    #[test]
    fn pascal_case_schema_names() {
        let kind = rule("type: naming_convention\ntarget: schema_name\nconvention: pascal_case\n");
        assert!(messages(&kind, CONTRACT).is_empty());
    }

    #[test]
    fn custom_pattern() {
        let kind = rule("type: naming_convention\ntarget: schema_physical_name\nconvention:\n  pattern: '^[a-z_]+$'\n");
        assert_eq!(
            messages(&kind, CONTRACT),
            vec!["schema physicalName `orders_v1` does not follow pattern `^[a-z_]+$`"]
        );
    }

    #[test]
    fn other_conventions() {
        let cases = [
            ("camel_case", "orderId", "OrderId"),
            ("kebab_case", "order-id", "order_id"),
            ("screaming_snake_case", "ORDER_ID", "order_id"),
        ];
        for (convention, good, bad) in cases {
            let kind = rule(&format!(
                "type: naming_convention\ntarget: property_name\nconvention: {convention}\n"
            ));
            let contract = format!("schema:\n  - name: t\n    properties:\n      - name: {good}\n      - name: {bad}\n");
            assert_eq!(messages(&kind, &contract).len(), 1, "{convention}");
        }
    }
}
