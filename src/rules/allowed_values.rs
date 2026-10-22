use std::collections::BTreeMap;

use serde::Deserialize;

use super::{default_true, list, Context, Violation};
use crate::config::pattern::Pattern;
use crate::config::suggest::lookup;
use crate::constants::keys;
use crate::document::node::{get, scalar_items, scalar_text};
use crate::document::Node;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "FieldSpec")]
#[remain::sorted]
pub enum ContractField {
    ApiVersion,
    ContractTags,
    CustomProperty(String),
    DataProduct,
    Domain,
    PropertyClassification,
    PropertyLogicalType,
    PropertyPhysicalType,
    PropertyTags,
    SchemaName,
    SchemaPhysicalType,
    SchemaTags,
    Status,
    TeamMembersRole,
    TeamMembersUsername,
    TeamName,
    Tenant,
}

const CUSTOM_PROPERTY_KEYS: [&str; 2] = ["customProperties", "custom_property"];

/// `(ODCS path, short name, field)`; both spellings are accepted in the config.
const NAMED_FIELDS: [(&str, &str, ContractField); 16] = [
    ("apiVersion", "api_version", ContractField::ApiVersion),
    ("domain", "domain", ContractField::Domain),
    ("status", "status", ContractField::Status),
    ("tenant", "tenant", ContractField::Tenant),
    ("dataProduct", "data_product", ContractField::DataProduct),
    ("tags", "contract_tags", ContractField::ContractTags),
    ("team.name", "team_name", ContractField::TeamName),
    (
        "team.members.username",
        "team_members_username",
        ContractField::TeamMembersUsername,
    ),
    (
        "team.members.role",
        "team_members_role",
        ContractField::TeamMembersRole,
    ),
    ("schema.name", "schema_name", ContractField::SchemaName),
    (
        "schema.physicalType",
        "schema_physical_type",
        ContractField::SchemaPhysicalType,
    ),
    ("schema.tags", "schema_tags", ContractField::SchemaTags),
    (
        "schema.properties.logicalType",
        "property_logical_type",
        ContractField::PropertyLogicalType,
    ),
    (
        "schema.properties.physicalType",
        "property_physical_type",
        ContractField::PropertyPhysicalType,
    ),
    (
        "schema.properties.classification",
        "property_classification",
        ContractField::PropertyClassification,
    ),
    (
        "schema.properties.tags",
        "property_tags",
        ContractField::PropertyTags,
    ),
];

#[derive(Deserialize)]
#[serde(untagged)]
enum FieldSpec {
    Name(String),
    Keyed(BTreeMap<String, String>),
}

impl TryFrom<FieldSpec> for ContractField {
    type Error = String;

    fn try_from(spec: FieldSpec) -> Result<Self, Self::Error> {
        match spec {
            FieldSpec::Name(name) => lookup(&name, &NAMED_FIELDS, "field"),
            FieldSpec::Keyed(map) => match map.into_iter().collect::<Vec<_>>().as_slice() {
                [(key, property)] if CUSTOM_PROPERTY_KEYS.contains(&key.as_str()) => {
                    Ok(Self::CustomProperty(property.clone()))
                }
                _ => {
                    Err("a keyed field must look like `{customProperties: <property>}`".to_string())
                }
            },
        }
    }
}

impl ContractField {
    fn label(&self) -> String {
        match self {
            Self::ApiVersion => keys::API_VERSION.to_string(),
            Self::ContractTags => keys::TAGS.to_string(),
            Self::CustomProperty(key) => format!("{}[{key}]", keys::CUSTOM_PROPERTIES),
            Self::DataProduct => keys::DATA_PRODUCT.to_string(),
            Self::Domain => keys::DOMAIN.to_string(),
            Self::PropertyClassification => format!("property {}", keys::CLASSIFICATION),
            Self::PropertyLogicalType => format!("property {}", keys::LOGICAL_TYPE),
            Self::PropertyPhysicalType => format!("property {}", keys::PHYSICAL_TYPE),
            Self::PropertyTags => format!("property {}", keys::TAGS),
            Self::SchemaName => format!("schema {}", keys::NAME),
            Self::SchemaPhysicalType => format!("schema {}", keys::PHYSICAL_TYPE),
            Self::SchemaTags => format!("schema {}", keys::TAGS),
            Self::Status => keys::STATUS.to_string(),
            Self::TeamMembersRole => format!("team member {}", keys::ROLE),
            Self::TeamMembersUsername => format!("team member {}", keys::USERNAME),
            Self::TeamName => format!("{}.{}", keys::TEAM, keys::NAME),
            Self::Tenant => keys::TENANT.to_string(),
        }
    }

    const fn is_team_member_field(&self) -> bool {
        matches!(self, Self::TeamMembersRole | Self::TeamMembersUsername)
    }

    /// The ODCS path users write for this field; `None` for custom properties, which need a key.
    pub fn path(&self) -> Option<&'static str> {
        NAMED_FIELDS
            .iter()
            .find(|(_, _, field)| field == self)
            .map(|(path, _, _)| *path)
    }
}

pub const FIELD_OPTION: &str = "field";
pub const ROLE_OPTION: &str = "role";
const OWNER_ROLE: &str = "owner";

/// A rule type that is `allowed_values` with its `field` (and `role`) already set.
#[derive(Debug)]
pub struct Alias {
    pub name: &'static str,
    pub field: ContractField,
    pub role: Option<&'static str>,
}

impl Alias {
    const fn new(name: &'static str, field: ContractField) -> Self {
        Self {
            name,
            field,
            role: None,
        }
    }

    /// The options this alias sets, as `(option, value)`.
    pub fn presets(&self) -> Vec<(&'static str, &'static str)> {
        let field = self.field.path().map(|path| (FIELD_OPTION, path));
        let role = self.role.map(|role| (ROLE_OPTION, role));
        field.into_iter().chain(role).collect()
    }
}

pub static ALIASES: [Alias; 15] = [
    Alias::new("allowed_api_versions", ContractField::ApiVersion),
    Alias::new("allowed_domains", ContractField::Domain),
    Alias::new("allowed_statuses", ContractField::Status),
    Alias::new("allowed_tenants", ContractField::Tenant),
    Alias::new("allowed_data_products", ContractField::DataProduct),
    Alias::new("allowed_tags", ContractField::ContractTags),
    Alias::new("allowed_teams", ContractField::TeamName),
    Alias::new("allowed_members", ContractField::TeamMembersUsername),
    Alias {
        name: "allowed_owners",
        field: ContractField::TeamMembersUsername,
        role: Some(OWNER_ROLE),
    },
    Alias::new("allowed_roles", ContractField::TeamMembersRole),
    Alias::new("allowed_schema_types", ContractField::SchemaPhysicalType),
    Alias::new("allowed_logical_types", ContractField::PropertyLogicalType),
    Alias::new(
        "allowed_physical_types",
        ContractField::PropertyPhysicalType,
    ),
    Alias::new(
        "allowed_classifications",
        ContractField::PropertyClassification,
    ),
    Alias::new("allowed_property_tags", ContractField::PropertyTags),
];

pub fn alias(name: &str) -> Option<&'static Alias> {
    ALIASES.iter().find(|alias| alias.name == name)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowedValues {
    pub field: ContractField,
    #[serde(default)]
    pub values: Vec<String>,
    pub pattern: Option<Pattern>,
    pub role: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
}

impl AllowedValues {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.values.is_empty() && self.pattern.is_none() {
            Err("`values` or `pattern` is required".to_string())
        } else if self.role.is_some() && !self.field.is_team_member_field() {
            Err("`role` only applies to team.members.username and team.members.role".to_string())
        } else {
            Ok(())
        }
    }

    fn equals(&self, left: &str, right: &str) -> bool {
        if self.case_sensitive {
            left == right
        } else {
            left.to_lowercase() == right.to_lowercase()
        }
    }

    fn allows(&self, value: &str) -> bool {
        self.values
            .iter()
            .any(|allowed| self.equals(allowed, value))
            || self
                .pattern
                .as_ref()
                .is_some_and(|pattern| pattern.is_match(value))
    }

    fn rejection(&self, label: &str, value: &str) -> String {
        match (self.values.is_empty(), &self.pattern) {
            (false, None) => format!(
                "{label} `{value}` is not allowed; allowed values: {}",
                list(&self.values)
            ),
            (true, Some(pattern)) => format!(
                "{label} `{value}` does not match pattern `{}`",
                pattern.as_str()
            ),
            (_, pattern) => format!(
                "{label} `{value}` is not allowed; allowed values: {} or pattern `{}`",
                list(&self.values),
                pattern.as_ref().map_or("", |pattern| pattern.as_str())
            ),
        }
    }
}

struct Target<'a> {
    owner: &'a Node<'a>,
    values: Vec<&'a Node<'a>>,
}

pub fn check(options: &AllowedValues, context: &Context<'_>) -> Vec<Violation> {
    let label = options.field.label();
    let targets = targets(options, context);
    let nothing_found = (options.required && targets.is_empty()).then(|| {
        Violation::new(
            format!("no {label} found"),
            context.location(context.contract.root()),
        )
    });
    let target_violations = targets.into_iter().flat_map(|target| {
        let missing = (options.required && target.values.is_empty()).then(|| {
            Violation::new(
                format!("{label} is missing"),
                context.location(target.owner),
            )
        });
        let rejected = target.values.into_iter().filter_map(|node| {
            let value = scalar_text(node)?;
            (!options.allows(&value))
                .then(|| Violation::new(options.rejection(&label, &value), context.location(node)))
        });
        missing.into_iter().chain(rejected).collect::<Vec<_>>()
    });
    nothing_found.into_iter().chain(target_violations).collect()
}

fn targets<'a>(options: &AllowedValues, context: &Context<'a>) -> Vec<Target<'a>> {
    let contract = context.contract;
    let root = contract.root();
    let at_root = |values: Vec<&'a Node<'a>>| {
        vec![Target {
            owner: root,
            values,
        }]
    };
    let field_values =
        |node: &'a Node<'a>, key: &str| get(node, key).map(scalar_items).unwrap_or_default();
    let schema_targets = |key: &str| {
        contract
            .schema_objects()
            .iter()
            .map(|object| Target {
                owner: object.node,
                values: field_values(object.node, key),
            })
            .collect()
    };
    let property_targets = |key: &str| {
        contract
            .properties()
            .iter()
            .map(|property| Target {
                owner: property.node,
                values: field_values(property.node, key),
            })
            .collect()
    };
    let member_targets = |key: &str| {
        contract
            .team_members()
            .into_iter()
            .filter(|member| {
                options.role.as_ref().is_none_or(|role| {
                    get(member, keys::ROLE)
                        .and_then(scalar_text)
                        .is_some_and(|member_role| options.equals(role, &member_role))
                })
            })
            .map(|member| Target {
                owner: member,
                values: field_values(member, key),
            })
            .collect()
    };

    match &options.field {
        ContractField::ApiVersion => at_root(field_values(root, keys::API_VERSION)),
        ContractField::ContractTags => at_root(field_values(root, keys::TAGS)),
        ContractField::CustomProperty(key) => at_root(
            contract
                .custom_property_values(key)
                .into_iter()
                .flat_map(scalar_items)
                .collect(),
        ),
        ContractField::DataProduct => at_root(field_values(root, keys::DATA_PRODUCT)),
        ContractField::Domain => at_root(field_values(root, keys::DOMAIN)),
        ContractField::PropertyClassification => property_targets(keys::CLASSIFICATION),
        ContractField::PropertyLogicalType => property_targets(keys::LOGICAL_TYPE),
        ContractField::PropertyPhysicalType => property_targets(keys::PHYSICAL_TYPE),
        ContractField::PropertyTags => property_targets(keys::TAGS),
        ContractField::SchemaName => schema_targets(keys::NAME),
        ContractField::SchemaPhysicalType => schema_targets(keys::PHYSICAL_TYPE),
        ContractField::SchemaTags => schema_targets(keys::TAGS),
        ContractField::Status => at_root(field_values(root, keys::STATUS)),
        ContractField::TeamMembersRole => member_targets(keys::ROLE),
        ContractField::TeamMembersUsername => member_targets(keys::USERNAME),
        ContractField::TeamName => {
            at_root(contract.team_name().map(scalar_items).unwrap_or_default())
        }
        ContractField::Tenant => at_root(field_values(root, keys::TENANT)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Location;
    use crate::rules::test_support::{assert_passes, messages, parse_rule, rule, violations_at};

    const TEAM: &str = "\
team:
  name: finance
  members:
    - username: alice
      role: owner
    - username: mallory
      role: owner
    - username: bob
      role: engineer
";

    #[test]
    fn every_alias_sets_a_named_field() {
        let unnamed: Vec<&str> = ALIASES
            .iter()
            .filter(|alias| alias.field.path().is_none())
            .map(|alias| alias.name)
            .collect();
        assert!(
            unnamed.is_empty(),
            "aliases without a field path: {unnamed:?}"
        );
    }

    #[test]
    fn owners_alias_presets_field_and_role() {
        let owners = alias("allowed_owners").unwrap();
        assert_eq!(
            owners.presets(),
            vec![("field", "team.members.username"), ("role", "owner")]
        );
    }

    #[test]
    fn accepts_odcs_paths_as_field_names() {
        let pairs = [
            ("apiVersion", "api_version"),
            ("team.members.username", "team_members_username"),
            ("schema.properties.logicalType", "property_logical_type"),
            ("{customProperties: owner}", "{custom_property: owner}"),
        ];
        for (odcs, snake) in pairs {
            let by_path = parse_rule(&format!(
                "type: allowed_values\nfield: {odcs}\nvalues: [x]\n"
            ));
            let by_name = parse_rule(&format!(
                "type: allowed_values\nfield: {snake}\nvalues: [x]\n"
            ));
            assert_eq!(format!("{by_path:?}"), format!("{by_name:?}"), "{odcs}");
        }
    }

    #[test]
    fn unknown_field_suggests_the_closest_name() {
        let error = serde_json::from_value::<crate::rules::RuleKind>(serde_json::json!(
            {"type": "allowed_values", "field": "domian", "values": ["x"]}
        ))
        .unwrap_err()
        .to_string();
        assert!(
            error.starts_with(
                "unknown field `domian`; did you mean `domain`? Expected one of `apiVersion`"
            ),
            "{error}"
        );
    }

    #[test]
    fn allows_listed_owner() {
        let kind = rule("type: allowed_values\nfield: team_members_username\nrole: owner\nvalues: [alice, mallory]\n");
        assert_passes(&kind, TEAM);
    }

    #[test]
    fn rejects_unlisted_owner_with_location() {
        let kind = rule(
            "type: allowed_values\nfield: team_members_username\nrole: owner\nvalues: [alice]\n",
        );
        let violations = violations_at(&kind, TEAM, "c.odcs.yaml");
        assert_eq!(
            violations,
            vec![Violation::new(
                "team member username `mallory` is not allowed; allowed values: alice".to_string(),
                Location {
                    line: 6,
                    column: 17
                },
            )]
        );
    }

    #[test]
    fn role_filter_skips_other_members() {
        let kind = rule("type: allowed_values\nfield: team_members_username\nrole: owner\nvalues: [alice, mallory]\n");
        assert!(!messages(&kind, TEAM)
            .iter()
            .any(|message| message.contains("bob")));
    }

    #[test]
    fn required_owner_missing_is_reported() {
        let kind = rule("type: allowed_values\nfield: team_members_username\nrole: owner\nvalues: [alice]\nrequired: true\n");
        assert_eq!(
            messages(
                &kind,
                "team:\n  members:\n    - username: bob\n      role: engineer\n"
            ),
            vec!["no team member username found"]
        );
    }

    #[test]
    fn checks_domain() {
        let kind = rule("type: allowed_values\nfield: domain\nvalues: [sales]\n");
        assert_eq!(
            messages(&kind, "domain: hr\n"),
            vec!["domain `hr` is not allowed; allowed values: sales"]
        );
    }

    #[test]
    fn missing_optional_field_is_fine() {
        let kind = rule("type: allowed_values\nfield: domain\nvalues: [sales]\n");
        assert_passes(&kind, "name: x\n");
    }

    #[test]
    fn missing_required_field_is_reported() {
        let kind = rule("type: allowed_values\nfield: domain\nvalues: [sales]\nrequired: true\n");
        assert_eq!(messages(&kind, "name: x\n"), vec!["domain is missing"]);
    }

    #[test]
    fn pattern_allows_values() {
        let kind = rule(
            "type: allowed_values\nfield: {custom_property: owner}\npattern: '^[a-z]+\\.[a-z]+$'\n",
        );
        let contract = "customProperties:\n  - property: owner\n    value: jane.doe\n  - property: owner\n    value: Bad\n";
        assert_eq!(
            messages(&kind, contract),
            vec!["customProperties[owner] `Bad` does not match pattern `^[a-z]+\\.[a-z]+$`"]
        );
    }

    #[test]
    fn case_insensitive_comparison() {
        let kind =
            rule("type: allowed_values\nfield: status\nvalues: [active]\ncase_sensitive: false\n");
        assert_passes(&kind, "status: ACTIVE\n");
    }

    #[test]
    fn checks_every_property_logical_type() {
        let kind =
            rule("type: allowed_values\nfield: property_logical_type\nvalues: [string, integer]\n");
        let contract = "schema:\n  - name: t\n    properties:\n      - name: a\n        logicalType: string\n      - name: b\n        logicalType: number\n";
        assert_eq!(
            messages(&kind, contract),
            vec!["property logicalType `number` is not allowed; allowed values: string, integer"]
        );
    }

    #[test]
    fn checks_each_tag() {
        let kind = rule("type: allowed_values\nfield: contract_tags\nvalues: [finance, pii]\n");
        assert_eq!(
            messages(&kind, "tags: [finance, secret]\n"),
            vec!["tags `secret` is not allowed; allowed values: finance, pii"]
        );
    }

    #[test]
    fn requires_values_or_pattern() {
        let kind = parse_rule("type: allowed_values\nfield: domain\n");
        assert_eq!(
            kind.validate(),
            Err("`values` or `pattern` is required".to_string())
        );
    }

    #[test]
    fn role_requires_team_member_field() {
        let kind = parse_rule("type: allowed_values\nfield: domain\nvalues: [a]\nrole: owner\n");
        assert!(kind.validate().is_err());
    }
}
