mod load;
pub mod pattern;

use std::path::Path;

use serde::Deserialize;

pub use load::{find, load};
use pattern::GlobList;

use crate::constants::DEFAULT_INCLUDE;
use crate::diagnostic::Severity;
use crate::rules::RuleKind;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        self.rules.iter().try_for_each(|rule| {
            rule.kind
                .validate()
                .map_err(|message| format!("rule `{}`: {message}", rule.display_name()))
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    pub include: GlobList,
    pub exclude: GlobList,
    pub schema_validation: SchemaValidation,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            include: GlobList::new(&DEFAULT_INCLUDE).unwrap_or_else(|_| GlobList::empty()),
            exclude: GlobList::empty(),
            schema_validation: SchemaValidation::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaValidation {
    #[default]
    Error,
    Warning,
    Off,
}

impl SchemaValidation {
    pub const fn severity(self) -> Option<Severity> {
        match self {
            Self::Error => Some(Severity::Error),
            Self::Warning => Some(Severity::Warning),
            Self::Off => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub name: Option<String>,
    #[serde(default)]
    pub severity: Severity,
    pub description: Option<String>,
    pub includes: Option<GlobList>,
    pub excludes: Option<GlobList>,
    #[serde(flatten)]
    pub kind: RuleKind,
}

impl Rule {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or_else(|| self.kind.as_ref())
    }

    pub fn applies_to(&self, path: &str) -> bool {
        let included = self
            .includes
            .as_ref()
            .is_none_or(|globs| globs.is_match(path));
        let excluded = self
            .excludes
            .as_ref()
            .is_some_and(|globs| globs.is_match(path));
        included && !excluded
    }
}

pub fn normalize_path(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    text.strip_prefix("./")
        .map_or_else(|| text.clone(), ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: serde_json::Value) -> Result<Config, serde_json::Error> {
        serde_json::from_value(json)
    }

    #[test]
    fn empty_config_uses_defaults() {
        let config = parse(serde_json::json!({})).unwrap();
        assert!(config.rules.is_empty());
        assert!(config.settings.include.is_match("a/b.odcs.yaml"));
        assert_eq!(config.settings.schema_validation, SchemaValidation::Error);
    }

    #[test]
    fn rule_defaults_to_error_severity_and_type_name() {
        let config =
            parse(serde_json::json!({"rules": [{"type": "unique_names", "scope": "property"}]}))
                .unwrap();
        assert_eq!(config.rules[0].severity, Severity::Error);
        assert_eq!(config.rules[0].display_name(), "unique_names");
    }

    #[test]
    fn rejects_unknown_rule_type() {
        let error = parse(serde_json::json!({"rules": [{"type": "nope"}]})).unwrap_err();
        assert!(error.to_string().contains("unknown variant `nope`"));
    }

    #[test]
    fn rejects_unknown_rule_option() {
        let error = parse(serde_json::json!({"rules": [{"type": "unique_names", "scope": "property", "typo": 1}]}))
            .unwrap_err();
        assert!(error.to_string().contains("unknown field `typo`"));
    }

    #[test]
    fn rejects_unknown_settings_key() {
        let error = parse(serde_json::json!({"settings": {"inclde": []}})).unwrap_err();
        assert!(error.to_string().contains("unknown field `inclde`"));
    }

    #[test]
    fn includes_and_excludes_filter_paths() {
        let config = parse(serde_json::json!({"rules": [{
            "type": "unique_names", "scope": "property",
            "includes": ["domains/**"], "excludes": ["domains/sandbox/**"]
        }]}))
        .unwrap();
        let rule = &config.rules[0];
        assert!(rule.applies_to("domains/sales/a.odcs.yaml"));
        assert!(!rule.applies_to("domains/sandbox/a.odcs.yaml"));
        assert!(!rule.applies_to("other/a.odcs.yaml"));
    }

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize_path(Path::new("./a/b.yaml")), "a/b.yaml");
        assert_eq!(normalize_path(Path::new("a\\b.yaml")), "a/b.yaml");
    }
}
