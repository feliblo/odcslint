//! Conversions for breaking changes between ODCS versions.
//!
//! Each migration looks at the contract's JSON and returns the operations still needed, so running
//! it again after a pass finds less to do.
#![expect(
    clippy::unnecessary_wraps,
    reason = "all migrations share one signature; the schema-driven one can fail"
)]

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use super::ops::{child, Op};
use crate::constants::keys;
use crate::error::Result;
use crate::schema::{declared_keys, SchemaVersion};

pub struct Migration {
    pub name: &'static str,
    pub plan: fn(&Value) -> Result<Vec<Op>>,
}

const ATHENA: &str = "athena";
const DUCKDB: &str = "duckdb";
const ROOT: &str = "";

/// Library rules from ODCS v3.0 with an exact v3.1 metric; other rules change meaning and are left to the user.
const RULE_METRICS: [(&str, &str); 2] = [
    ("rowCount", "rowCount"),
    ("duplicateCount", "duplicateValues"),
];

const OTHER_OPERATORS: [&str; 6] = [
    keys::MUST_BE,
    keys::MUST_NOT_BE,
    keys::MUST_BE_GREATER_THAN,
    keys::MUST_BE_LESS_THAN,
    keys::MUST_BE_BETWEEN,
    keys::MUST_NOT_BE_BETWEEN,
];

const BOUNDS: [(&str, &str); 2] = [
    (keys::EXCLUSIVE_MINIMUM, keys::MINIMUM),
    (keys::EXCLUSIVE_MAXIMUM, keys::MAXIMUM),
];

const TO_V3_0_2: [Migration; 1] = [Migration {
    name: "athena_staging_dir",
    plan: athena_staging_dir,
}];

/// Unknown keys are moved last, after the other migrations have reshaped the contract.
const TO_V3_1_0: [Migration; 8] = [
    Migration {
        name: "team_members",
        plan: team_members,
    },
    Migration {
        name: "exclusive_bounds",
        plan: exclusive_bounds,
    },
    Migration {
        name: "duckdb_schema_string",
        plan: duckdb_schema_string,
    },
    Migration {
        name: "sla_default_element",
        plan: sla_default_element,
    },
    Migration {
        name: "quality_metric",
        plan: quality_metric,
    },
    Migration {
        name: "quality_between",
        plan: quality_between,
    },
    Migration {
        name: "custom_property_value",
        plan: custom_property_value,
    },
    Migration {
        name: "unknown_keys_to_custom_properties",
        plan: unknown_keys_to_custom_properties,
    },
];

/// The migrations that move a contract onto `version` from the version before it.
pub const fn for_step(version: SchemaVersion) -> &'static [Migration] {
    match version {
        SchemaVersion::V3_0_2 => &TO_V3_0_2,
        SchemaVersion::V3_1_0 => &TO_V3_1_0,
        SchemaVersion::V3_0_0 | SchemaVersion::V3_0_1 | SchemaVersion::V3_2_0 => &[],
    }
}

fn walk<'a>(value: &'a Value, pointer: &str, found: &mut Vec<(String, &'a Value)>) {
    found.push((pointer.to_string(), value));
    match value {
        Value::Object(fields) => fields
            .iter()
            .for_each(|(key, child_value)| walk(child_value, &child(pointer, key), found)),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .for_each(|(index, item)| walk(item, &child(pointer, &index.to_string()), found)),
        _ => {}
    }
}

fn under_key<'a>(json: &'a Value, key: &str) -> Vec<(String, &'a Value)> {
    let mut found = Vec::new();
    walk(json, ROOT, &mut found);
    let suffix = child(ROOT, key);
    found
        .into_iter()
        .filter(|(pointer, _)| pointer.ends_with(&suffix))
        .collect()
}

fn items_under<'a>(json: &'a Value, key: &str) -> Vec<(String, &'a Map<String, Value>)> {
    under_key(json, key)
        .into_iter()
        .filter_map(|(pointer, value)| value.as_array().map(|items| (pointer, items)))
        .flat_map(|(pointer, items)| {
            items.iter().enumerate().filter_map(move |(index, item)| {
                item.as_object()
                    .map(|fields| (child(&pointer, &index.to_string()), fields))
            })
        })
        .collect()
}

fn servers_of_type<'a>(
    json: &'a Value,
    server_type: &str,
) -> Vec<(String, &'a Map<String, Value>)> {
    items_under(json, keys::SERVERS)
        .into_iter()
        .filter(|(pointer, fields)| {
            pointer.starts_with(&child(ROOT, keys::SERVERS))
                && fields.get(keys::TYPE).and_then(Value::as_str) == Some(server_type)
        })
        .collect()
}

fn athena_staging_dir(json: &Value) -> Result<Vec<Op>> {
    Ok(servers_of_type(json, ATHENA)
        .into_iter()
        .filter(|(_, fields)| fields.contains_key(keys::STAGING_DIR_LEGACY))
        .map(|(object, fields)| {
            if fields.contains_key(keys::STAGING_DIR) {
                Op::Remove {
                    object,
                    key: keys::STAGING_DIR_LEGACY.to_string(),
                }
            } else {
                Op::Rename {
                    object,
                    from: keys::STAGING_DIR_LEGACY.to_string(),
                    to: keys::STAGING_DIR.to_string(),
                }
            }
        })
        .collect())
}

fn team_members(json: &Value) -> Result<Vec<Op>> {
    Ok(json
        .get(keys::TEAM)
        .filter(|team| team.is_array())
        .map(|_| Op::Wrap {
            pointer: child(ROOT, keys::TEAM),
            key: keys::MEMBERS.to_string(),
        })
        .into_iter()
        .collect())
}

fn exclusive_bounds(json: &Value) -> Result<Vec<Op>> {
    Ok(under_key(json, keys::LOGICAL_TYPE_OPTIONS)
        .into_iter()
        .filter_map(|(pointer, value)| value.as_object().map(|fields| (pointer, fields)))
        .flat_map(|(pointer, fields)| {
            let ops: Vec<Op> = BOUNDS
                .iter()
                .flat_map(
                    |(exclusive, bound)| match (fields.get(*exclusive), fields.get(*bound)) {
                        (Some(Value::Bool(true)), Some(limit)) => vec![
                            Op::Set {
                                pointer: child(&pointer, exclusive),
                                value: limit.clone(),
                            },
                            Op::Remove {
                                object: pointer.clone(),
                                key: (*bound).to_string(),
                            },
                        ],
                        (Some(Value::Bool(_)), _) => {
                            vec![Op::Remove {
                                object: pointer.clone(),
                                key: (*exclusive).to_string(),
                            }]
                        }
                        _ => Vec::new(),
                    },
                )
                .collect();
            let removed: BTreeSet<&str> = ops
                .iter()
                .filter_map(|op| match op {
                    Op::Remove { key, .. } => Some(key.as_str()),
                    _ => None,
                })
                .collect();
            let empties =
                !ops.is_empty() && fields.keys().all(|key| removed.contains(key.as_str()));
            // An emptied `logicalTypeOptions:` would read back as null, so drop the key itself.
            match (empties, pointer.rsplit_once('/')) {
                (true, Some((parent, _))) => vec![Op::Remove {
                    object: parent.to_string(),
                    key: keys::LOGICAL_TYPE_OPTIONS.to_string(),
                }],
                _ => ops,
            }
        })
        .collect())
}

fn duckdb_schema_string(json: &Value) -> Result<Vec<Op>> {
    Ok(servers_of_type(json, DUCKDB)
        .into_iter()
        .filter_map(|(pointer, fields)| {
            fields
                .get(keys::SCHEMA)
                .and_then(Value::as_number)
                .map(|number| Op::Set {
                    pointer: child(&pointer, keys::SCHEMA),
                    value: Value::String(number.to_string()),
                })
        })
        .collect())
}

fn sla_default_element(json: &Value) -> Result<Vec<Op>> {
    let Some(element) = json.get(keys::SLA_DEFAULT_ELEMENT) else {
        return Ok(Vec::new());
    };
    let Some(slas) = json.get(keys::SLA_PROPERTIES).and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let sla_pointer = child(ROOT, keys::SLA_PROPERTIES);
    let inserts = slas.iter().enumerate().filter_map(|(index, sla)| {
        let fields = sla.as_object()?;
        (!fields.contains_key(keys::ELEMENT)).then(|| Op::Insert {
            object: child(&sla_pointer, &index.to_string()),
            key: keys::ELEMENT.to_string(),
            value: element.clone(),
        })
    });
    Ok(inserts
        .chain(std::iter::once(Op::Remove {
            object: ROOT.to_string(),
            key: keys::SLA_DEFAULT_ELEMENT.to_string(),
        }))
        .collect())
}

fn quality_metric(json: &Value) -> Result<Vec<Op>> {
    Ok(items_under(json, keys::QUALITY)
        .into_iter()
        .filter(|(_, fields)| !fields.contains_key(keys::METRIC))
        .filter_map(|(object, fields)| {
            let rule = fields.get(keys::RULE).and_then(Value::as_str)?;
            let (_, metric) = RULE_METRICS.iter().find(|(name, _)| *name == rule)?;
            Some(vec![
                Op::Insert {
                    object: object.clone(),
                    key: keys::METRIC.to_string(),
                    value: json!(metric),
                },
                Op::Remove {
                    object,
                    key: keys::RULE.to_string(),
                },
            ])
        })
        .flatten()
        .collect())
}

fn quality_between(json: &Value) -> Result<Vec<Op>> {
    Ok(items_under(json, keys::QUALITY)
        .into_iter()
        .filter(|(_, fields)| {
            !OTHER_OPERATORS
                .iter()
                .any(|operator| fields.contains_key(*operator))
        })
        .filter_map(|(object, fields)| {
            let low = fields.get(keys::MUST_BE_GREATER_OR_EQUAL_TO)?;
            let high = fields.get(keys::MUST_BE_LESS_OR_EQUAL_TO)?;
            let insert = if low == high {
                Op::Insert {
                    object: object.clone(),
                    key: keys::MUST_BE.to_string(),
                    value: low.clone(),
                }
            } else if low.as_f64()? < high.as_f64()? {
                Op::Insert {
                    object: object.clone(),
                    key: keys::MUST_BE_BETWEEN.to_string(),
                    value: json!([low, high]),
                }
            } else {
                return None;
            };
            Some(vec![
                insert,
                Op::Remove {
                    object: object.clone(),
                    key: keys::MUST_BE_GREATER_OR_EQUAL_TO.to_string(),
                },
                Op::Remove {
                    object,
                    key: keys::MUST_BE_LESS_OR_EQUAL_TO.to_string(),
                },
            ])
        })
        .flatten()
        .collect())
}

fn custom_property_value(json: &Value) -> Result<Vec<Op>> {
    Ok(items_under(json, keys::CUSTOM_PROPERTIES)
        .into_iter()
        .filter(|(_, fields)| {
            fields.contains_key(keys::PROPERTY) && !fields.contains_key(keys::VALUE)
        })
        .map(|(object, _)| Op::Insert {
            object,
            key: keys::VALUE.to_string(),
            value: Value::Null,
        })
        .collect())
}

/// Objects whose schema definition has `customProperties`, with that definition's name.
fn extensible_objects(json: &Value) -> Vec<(String, &'static str)> {
    let list = |key: &str, definition: &'static str| {
        json.get(key)
            .and_then(Value::as_array)
            .map(|items| {
                (0..items.len())
                    .map(|index| (child(&child(ROOT, key), &index.to_string()), definition))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let team = match json.get(keys::TEAM) {
        Some(Value::Object(fields)) => std::iter::once((child(ROOT, keys::TEAM), "Team"))
            .chain(
                fields
                    .get(keys::MEMBERS)
                    .and_then(Value::as_array)
                    .map(|members| {
                        (0..members.len())
                            .map(|index| {
                                let members_pointer =
                                    child(&child(ROOT, keys::TEAM), keys::MEMBERS);
                                (child(&members_pointer, &index.to_string()), "TeamMember")
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
            .collect(),
        Some(Value::Array(_)) => list(keys::TEAM, "TeamMember"),
        _ => Vec::new(),
    };
    let schema_objects = list(keys::SCHEMA, "SchemaObject");
    let properties = under_key(json, keys::PROPERTIES)
        .into_iter()
        .filter(|(pointer, _)| pointer.starts_with(&child(ROOT, keys::SCHEMA)))
        .filter_map(|(pointer, value)| value.as_array().map(|items| (pointer, items.len())))
        .flat_map(|(pointer, count)| {
            (0..count).map(move |index| (child(&pointer, &index.to_string()), "SchemaProperty"))
        });
    let quality = items_under(json, keys::QUALITY)
        .into_iter()
        .map(|(pointer, _)| (pointer, "DataQuality"));
    list(keys::SERVERS, "Server")
        .into_iter()
        .chain(list(keys::ROLES, "Role"))
        .chain(list(keys::SUPPORT, "SupportItem"))
        .chain(team)
        .chain(schema_objects)
        .chain(properties)
        .chain(quality)
        .collect()
}

/// v3.1.0 rejects keys its schema doesn't declare. Keys unknown to the schema are moved into the
/// object's `customProperties`, where ODCS allows arbitrary data, so nothing is lost.
fn unknown_keys_to_custom_properties(json: &Value) -> Result<Vec<Op>> {
    extensible_objects(json)
        .into_iter()
        .map(|(pointer, definition)| {
            let declared = declared_keys(SchemaVersion::V3_1_0, definition)?;
            let Some(fields) = json.pointer(&pointer).and_then(Value::as_object) else {
                return Ok(Vec::new());
            };
            if !declared.contains(keys::CUSTOM_PROPERTIES) {
                return Ok(Vec::new());
            }
            let unknown: Vec<(&String, &Value)> = fields
                .iter()
                .filter(|(key, _)| !declared.contains(key.as_str()))
                .collect();
            if unknown.is_empty() {
                return Ok(Vec::new());
            }
            let entries: Vec<Value> = unknown
                .iter()
                .map(|(key, value)| json!({keys::PROPERTY: key, keys::VALUE: value}))
                .collect();
            let add = match fields.get(keys::CUSTOM_PROPERTIES) {
                Some(Value::Array(existing)) if !existing.is_empty() => entries
                    .into_iter()
                    .map(|value| Op::Append {
                        array: child(&pointer, keys::CUSTOM_PROPERTIES),
                        value,
                    })
                    .collect(),
                Some(_) => Vec::new(),
                None => vec![Op::Insert {
                    object: pointer.clone(),
                    key: keys::CUSTOM_PROPERTIES.to_string(),
                    value: Value::Array(entries),
                }],
            };
            if add.is_empty() {
                return Ok(Vec::new());
            }
            let removes = unknown.iter().map(|(key, _)| Op::Remove {
                object: pointer.clone(),
                key: (*key).clone(),
            });
            Ok(add.into_iter().chain(removes).collect())
        })
        .collect::<Result<Vec<Vec<Op>>>>()
        .map(|ops| ops.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(migration: fn(&Value) -> Result<Vec<Op>>, json: Value) -> Value {
        let ops = migration(&json).unwrap();
        ops.iter().fold(json, |mut value, op| {
            op.apply(&mut value).unwrap();
            value
        })
    }

    #[test]
    fn renames_athena_staging_dir() {
        let result = plan(
            athena_staging_dir,
            json!({"servers": [{"type": "athena", "staging_dir": "s3://x", "schema": "db"}]}),
        );
        assert_eq!(
            result,
            json!({"servers": [{"type": "athena", "stagingDir": "s3://x", "schema": "db"}]})
        );
    }

    #[test]
    fn drops_staging_dir_when_both_exist() {
        let result = plan(
            athena_staging_dir,
            json!({"servers": [{"type": "athena", "staging_dir": "a", "stagingDir": "b"}]}),
        );
        assert_eq!(
            result,
            json!({"servers": [{"type": "athena", "stagingDir": "b"}]})
        );
    }

    #[test]
    fn wraps_team_list_in_members() {
        let result = plan(team_members, json!({"team": [{"username": "a"}]}));
        assert_eq!(result, json!({"team": {"members": [{"username": "a"}]}}));
        assert_eq!(team_members(&result).unwrap(), Vec::<Op>::new());
    }

    #[test]
    fn turns_exclusive_flags_into_bounds() {
        let result = plan(
            exclusive_bounds,
            json!({"schema": [{"properties": [{"logicalTypeOptions": {"maximum": 10, "exclusiveMaximum": true, "minimum": 0, "exclusiveMinimum": false}}]}]}),
        );
        assert_eq!(
            result,
            json!({"schema": [{"properties": [{"logicalTypeOptions": {"exclusiveMaximum": 10, "minimum": 0}}]}]})
        );
    }

    #[test]
    fn removes_logical_type_options_left_empty() {
        let result = plan(
            exclusive_bounds,
            json!({"schema": [{"properties": [{"name": "a", "logicalTypeOptions": {"exclusiveMaximum": false}}]}]}),
        );
        assert_eq!(result, json!({"schema": [{"properties": [{"name": "a"}]}]}));
    }

    #[test]
    fn duckdb_schema_becomes_a_string() {
        let result = plan(
            duckdb_schema_string,
            json!({"servers": [{"type": "duckdb", "schema": 1}]}),
        );
        assert_eq!(
            result,
            json!({"servers": [{"type": "duckdb", "schema": "1"}]})
        );
    }

    #[test]
    fn copies_sla_default_element_to_each_sla() {
        let result = plan(
            sla_default_element,
            json!({"slaDefaultElement": "tbl", "slaProperties": [{"property": "latency"}, {"property": "retention", "element": "tbl.col"}]}),
        );
        assert_eq!(
            result,
            json!({"slaProperties": [{"property": "latency", "element": "tbl"}, {"property": "retention", "element": "tbl.col"}]})
        );
    }

    #[test]
    fn maps_rules_with_an_exact_metric() {
        let result = plan(
            quality_metric,
            json!({"schema": [{"quality": [{"rule": "duplicateCount", "mustBe": 0}, {"rule": "validValues", "mustBe": 0}]}]}),
        );
        assert_eq!(
            result,
            json!({"schema": [{"quality": [{"metric": "duplicateValues", "mustBe": 0}, {"rule": "validValues", "mustBe": 0}]}]})
        );
    }

    #[test]
    fn merges_inclusive_bounds_into_between() {
        let result = plan(
            quality_between,
            json!({"schema": [{"quality": [{"mustBeGreaterOrEqualTo": 1, "mustBeLessOrEqualTo": 5}, {"mustBeGreaterOrEqualTo": 3, "mustBeLessOrEqualTo": 3}]}]}),
        );
        assert_eq!(
            result,
            json!({"schema": [{"quality": [{"mustBeBetween": [1, 5]}, {"mustBe": 3}]}]})
        );
    }

    #[test]
    fn adds_missing_custom_property_values() {
        let result = plan(
            custom_property_value,
            json!({"customProperties": [{"property": "owner"}]}),
        );
        assert_eq!(
            result,
            json!({"customProperties": [{"property": "owner", "value": null}]})
        );
    }

    #[test]
    fn moves_unknown_keys_into_custom_properties() {
        let result = plan(
            unknown_keys_to_custom_properties,
            json!({"servers": [{"server": "s", "type": "postgres", "host": "h", "port": 5432, "database": "d", "schema": "p", "comment": "legacy"}]}),
        );
        assert_eq!(
            result,
            json!({"servers": [{"server": "s", "type": "postgres", "host": "h", "port": 5432, "database": "d", "schema": "p",
                "customProperties": [{"property": "comment", "value": "legacy"}]}]})
        );
    }

    #[test]
    fn appends_to_existing_custom_properties() {
        let result = plan(
            unknown_keys_to_custom_properties,
            json!({"team": {"members": [{"username": "a", "email": "a@x", "customProperties": [{"property": "p", "value": 1}]}]}}),
        );
        assert_eq!(
            result,
            json!({"team": {"members": [{"username": "a", "customProperties": [{"property": "p", "value": 1}, {"property": "email", "value": "a@x"}]}]}})
        );
    }

    #[test]
    fn keeps_declared_keys() {
        let json = json!({"schema": [{"name": "t", "properties": [{"name": "c", "logicalType": "string", "primaryKey": true}]}]});
        assert_eq!(
            unknown_keys_to_custom_properties(&json).unwrap(),
            Vec::<Op>::new()
        );
    }
}
