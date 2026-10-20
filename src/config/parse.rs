use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::suggest::{unknown_name, with_suggestion};
use super::{Config, Settings};
use crate::config::Rule;
use crate::constants::PYPROJECT_FILE_NAME;
use crate::diagnostic::Location;
use crate::document::node::{entries, entry, scalar_text};
use crate::document::pointer::resolve;
use crate::document::to_json::to_json;
use crate::document::Document;

const TOML_EXTENSION: &str = "toml";
const SETTINGS_KEY: &str = "settings";
const RULES_KEY: &str = "rules";
const TYPE_KEY: &str = "type";
const TOP_LEVEL_KEYS: [&str; 2] = [SETTINGS_KEY, RULES_KEY];
const PYPROJECT_SECTION: [&str; 2] = ["tool", "odcslint"];
const PYPROJECT_PREFIX: &str = "tool.odcslint.";

/// Why a config could not be loaded, and where in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub message: String,
    pub location: Option<Location>,
}

impl Invalid {
    fn new(message: impl Into<String>, location: Option<Location>) -> Self {
        Self {
            message: message.into(),
            location,
        }
    }
}

/// Points a JSON pointer (and optionally a key inside it) back at the config source.
trait Locate {
    fn locate(&self, pointer: &str, key: Option<&str>) -> Option<Location>;
}

struct YamlSource<'a>(&'a Document<'a>);

impl Locate for YamlSource<'_> {
    fn locate(&self, pointer: &str, key: Option<&str>) -> Option<Location> {
        let target = resolve(self.0.root(), pointer);
        let node = key
            .and_then(|name| {
                entry(target.node, name).or_else(|| {
                    entries(target.node)
                        .find(|(_, value)| scalar_text(value).as_deref() == Some(name))
                })
            })
            .map_or_else(|| target.anchor(), |(key, _)| key);
        Some(self.0.location(node))
    }
}

/// TOML parsing keeps no positions, so rules and settings are found by their table headers.
struct TomlSource<'a> {
    source: &'a str,
    prefix: &'a str,
}

impl Locate for TomlSource<'_> {
    fn locate(&self, pointer: &str, key: Option<&str>) -> Option<Location> {
        let lines: Vec<&str> = self.source.lines().collect();
        let segments: Vec<&str> = pointer.split('/').skip(1).collect();
        let header = match segments.as_slice() {
            [RULES_KEY, index] => {
                let header = format!("[[{}{RULES_KEY}]]", self.prefix);
                lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| line.trim() == header)
                    .nth(index.parse().ok()?)
                    .map(|(number, _)| number)
            }
            [SETTINGS_KEY] => {
                let header = format!("[{}{SETTINGS_KEY}]", self.prefix);
                lines.iter().position(|line| line.trim() == header)
            }
            _ => None,
        }?;
        let key_line = key.and_then(|key| {
            lines[header + 1..]
                .iter()
                .take_while(|line| !line.trim_start().starts_with('['))
                .position(|line| {
                    let line = line.trim();
                    let is_key = line
                        .strip_prefix(key)
                        .is_some_and(|rest| rest.trim_start().starts_with('='));
                    let has_value = line.ends_with(&format!("\"{key}\""));
                    is_key || has_value
                })
                .map(|offset| header + 1 + offset)
        });
        let line = key_line.unwrap_or(header);
        let indent = lines[line].len() - lines[line].trim_start().len();
        Some(Location {
            line: line + 1,
            column: indent + 1,
        })
    }
}

pub fn parse(path: &Path, source: &str) -> Result<Config, Invalid> {
    let is_pyproject = path
        .file_name()
        .is_some_and(|name| name == PYPROJECT_FILE_NAME);
    let is_toml = path
        .extension()
        .is_some_and(|extension| extension == TOML_EXTENSION);
    if is_pyproject {
        let table = parse_toml(source)?;
        let section = PYPROJECT_SECTION
            .iter()
            .try_fold(&table, |value, key| value.get(key));
        let json = section.map_or_else(|| Ok(Value::Object(Map::new())), toml_to_json)?;
        from_json(
            &json,
            &TomlSource {
                source,
                prefix: PYPROJECT_PREFIX,
            },
        )
    } else if is_toml {
        let json = toml_to_json(&parse_toml(source)?)?;
        from_json(&json, &TomlSource { source, prefix: "" })
    } else if source.trim().is_empty() {
        Ok(Config::default())
    } else {
        let document = Document::parse(source)
            .map_err(|error| Invalid::new(error.message, Some(error.location)))?;
        from_json(&to_json(document.root()), &YamlSource(&document))
    }
}

fn parse_toml(source: &str) -> Result<toml::Value, Invalid> {
    toml::from_str(source).map_err(|error| {
        let location = error.span().map(|span| location_of(source, span.start));
        Invalid::new(error.message().trim(), location)
    })
}

fn toml_to_json(value: &toml::Value) -> Result<Value, Invalid> {
    serde_json::to_value(value).map_err(|error| Invalid::new(error.to_string(), None))
}

fn location_of(source: &str, offset: usize) -> Location {
    let before = &source[..offset.min(source.len())];
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    Location {
        line: before.matches('\n').count() + 1,
        column: before[line_start..].chars().count() + 1,
    }
}

/// Parses settings and every rule separately, so an error can name the rule and its position.
fn from_json(json: &Value, source: &impl Locate) -> Result<Config, Invalid> {
    let Some(object) = json.as_object() else {
        return Err(Invalid::new(
            "the config must be a mapping",
            source.locate("", None),
        ));
    };
    if let Some(unknown) = object
        .keys()
        .find(|key| !TOP_LEVEL_KEYS.contains(&key.as_str()))
    {
        let message =
            format!("unknown key `{unknown}`, expected one of `{SETTINGS_KEY}`, `{RULES_KEY}`");
        return Err(Invalid::new(
            with_suggestion(&message),
            source.locate("", Some(unknown)),
        ));
    }
    let settings = object
        .get(SETTINGS_KEY)
        .map(|value| {
            deserialize::<Settings>(value).map_err(|message| {
                let location = source.locate(&format!("/{SETTINGS_KEY}"), unknown_name(&message));
                Invalid::new(format!("{SETTINGS_KEY}: {message}"), location)
            })
        })
        .transpose()?
        .unwrap_or_default();
    let rules = match object.get(RULES_KEY) {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| parse_rule(index, item, source))
            .collect::<Result<_, _>>()?,
        Some(_) => {
            return Err(Invalid::new(
                format!("`{RULES_KEY}` must be a list"),
                source.locate("", Some(RULES_KEY)),
            ))
        }
    };
    Ok(Config {
        settings,
        rules,
        root: PathBuf::new(),
    })
}

fn parse_rule(index: usize, item: &Value, source: &impl Locate) -> Result<Rule, Invalid> {
    let pointer = format!("/{RULES_KEY}/{index}");
    let label = item.get(TYPE_KEY).and_then(Value::as_str).map_or_else(
        || format!("{RULES_KEY}[{index}]"),
        |kind| format!("{RULES_KEY}[{index}] ({kind})"),
    );
    let rule: Rule = deserialize(item).map_err(|message| {
        let location = source.locate(&pointer, unknown_name(&message));
        Invalid::new(format!("{label}: {message}"), location)
    })?;
    rule.kind.validate().map_err(|message| {
        Invalid::new(format!("{label}: {message}"), source.locate(&pointer, None))
    })?;
    Ok(rule)
}

fn deserialize<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    T::deserialize(value).map_err(|error| with_suggestion(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml_error(source: &str) -> Invalid {
        parse(Path::new("odcslint.yml"), source).unwrap_err()
    }

    fn toml_error(source: &str) -> Invalid {
        parse(Path::new("odcslint.toml"), source).unwrap_err()
    }

    #[test]
    fn names_rule_points_at_key_and_suggests() {
        let error = yaml_error(
            "rules:\n  - type: primary_key\n  - type: allowed_values\n    field: domain\n    valuse: [sales]\n",
        );
        assert_eq!(
            error.message,
            "rules[1] (allowed_values): unknown field `valuse`, expected one of `field`, `values`, \
             `pattern`, `role`, `required`, `case_sensitive`; did you mean `values`?"
        );
        assert_eq!(error.location, Some(Location { line: 5, column: 5 }));
    }

    #[test]
    fn unknown_value_points_at_its_key() {
        let error =
            yaml_error("rules:\n  - type: allowed_values\n    field: domian\n    values: [x]\n");
        assert!(error.message.contains("did you mean `domain`?"));
        assert_eq!(error.location, Some(Location { line: 3, column: 5 }));
    }

    #[test]
    fn unknown_value_points_at_its_toml_key() {
        let error = toml_error(
            "[[rules]]\ntype = \"allowed_values\"\nfield = \"domian\"\nvalues = [\"x\"]\n",
        );
        assert_eq!(error.location, Some(Location { line: 3, column: 1 }));
    }

    #[test]
    fn suggests_rule_type() {
        let error = yaml_error("rules:\n  - type: allowed_value\n    field: domain\n");
        assert!(error
            .message
            .starts_with("rules[0] (allowed_value): unknown variant `allowed_value`"));
        assert!(error.message.ends_with("did you mean `allowed_values`?"));
        assert_eq!(error.location, Some(Location { line: 2, column: 5 }));
    }

    #[test]
    fn reports_semantic_errors_with_rule_location() {
        let error = yaml_error("rules:\n  - type: allowed_values\n    field: domain\n");
        assert_eq!(
            error.message,
            "rules[0] (allowed_values): `values` or `pattern` is required"
        );
        assert_eq!(error.location, Some(Location { line: 2, column: 5 }));
    }

    #[test]
    fn reports_unknown_settings_key() {
        let error = yaml_error("settings:\n  exclud: []\n");
        assert!(error
            .message
            .starts_with("settings: unknown field `exclud`"));
        assert!(error.message.ends_with("did you mean `exclude`?"));
        assert_eq!(error.location, Some(Location { line: 2, column: 3 }));
    }

    #[test]
    fn reports_unknown_top_level_key() {
        let error = yaml_error("rule:\n  - type: primary_key\n");
        assert_eq!(
            error.message,
            "unknown key `rule`, expected one of `settings`, `rules`; did you mean `rules`?"
        );
        assert_eq!(error.location, Some(Location { line: 1, column: 1 }));
    }

    #[test]
    fn reports_yaml_syntax_errors_with_location() {
        let error = yaml_error("rules:\n  - type: [\n");
        assert!(error.location.is_some());
    }

    #[test]
    fn locates_toml_rule_keys() {
        let error = toml_error(
            "[[rules]]\ntype = \"primary_key\"\n\n[[rules]]\ntype = \"allowed_values\"\nfield = \"domain\"\nvaluse = [\"sales\"]\n",
        );
        assert!(error
            .message
            .starts_with("rules[1] (allowed_values): unknown field `valuse`"));
        assert_eq!(error.location, Some(Location { line: 7, column: 1 }));
    }

    #[test]
    fn locates_pyproject_rules() {
        let error = parse(
            Path::new("pyproject.toml"),
            "[project]\nname = \"x\"\n\n[[tool.odcslint.rules]]\ntype = \"nope\"\n",
        )
        .unwrap_err();
        assert!(error
            .message
            .starts_with("rules[0] (nope): unknown variant `nope`"));
        assert_eq!(error.location, Some(Location { line: 5, column: 1 }));
    }

    #[test]
    fn reports_toml_syntax_errors_with_location() {
        let error = toml_error("[[rules]]\ntype = \n");
        assert_eq!(error.location.map(|location| location.line), Some(2));
    }

    #[test]
    fn valid_config_parses() {
        let config = parse(
            Path::new("odcslint.yml"),
            "settings:\n  exclude: ['x/**']\nrules:\n  - type: primary_key\n",
        )
        .unwrap();
        assert_eq!(config.rules.len(), 1);
    }
}
