//! Nested properties are validated level by level instead of as one instance.
//!
//! The ODCS schemas recurse through `if`/`then` branches under `unevaluatedProperties`, and
//! validators re-run those branches to find out which keys they evaluated. Every nesting level
//! doubles the work, so a contract with deep properties takes exponential time. Here each
//! recursion edge is cut: the parent is validated with a valid stand-in at the cut, and the
//! children are validated on their own against the definition the edge refers to.

use serde_json::{Map, Value};
use strum::EnumIter;

use super::diagnostics::is_within;
use crate::constants::keys;

const MAP_KEY: &str = "map";
const MAP_ENTRY_KEYS: [&str; 2] = ["key", "value"];

/// The schema a part is validated against: the whole document, or an entry of `$defs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum Definition {
    Contract,
    SchemaProperty,
    SchemaItemProperty,
}

impl Definition {
    pub const COUNT: usize = 3;

    pub const fn name(self) -> Option<&'static str> {
        match self {
            Self::Contract => None,
            Self::SchemaProperty => Some("SchemaProperty"),
            Self::SchemaItemProperty => Some("SchemaItemProperty"),
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A list of properties is cut as a whole; `items` and map entries are single properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    List,
    Single,
}

impl Shape {
    /// Valid wherever the full subtree would be evaluated, so the parent's own errors are kept.
    fn stand_in(self) -> Value {
        match self {
            Self::List => Value::Array(Vec::new()),
            Self::Single => Value::Object(Map::new()),
        }
    }

    /// Fails wherever the subtree would be evaluated, which tells whether to descend into it.
    fn probe(self) -> Value {
        match self {
            Self::List => Value::Array(vec![Value::Null]),
            Self::Single => Value::Null,
        }
    }

    const fn definition(self) -> Definition {
        match self {
            Self::List => Definition::SchemaProperty,
            Self::Single => Definition::SchemaItemProperty,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Part<'a> {
    pub value: &'a Value,
    pub definition: Definition,
}

#[derive(Debug, Clone)]
pub struct Cut<'a> {
    /// Relative to the part.
    pub pointer: String,
    shape: Shape,
    value: &'a Value,
}

impl<'a> Cut<'a> {
    /// The parts below this cut, with pointers relative to the part that holds the cut.
    pub fn children(&self) -> Vec<(String, Part<'a>)> {
        let definition = self.shape.definition();
        match (self.shape, self.value) {
            (Shape::List, Value::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    (
                        format!("{}/{index}", self.pointer),
                        Part { value, definition },
                    )
                })
                .collect(),
            (Shape::List | Shape::Single, value) => {
                vec![(self.pointer.clone(), Part { value, definition })]
            }
        }
    }
}

/// The recursion edges of a part. Only arrays and objects are cut, so type errors at an edge
/// are still reported by the parent.
pub fn cuts(part: Part<'_>) -> Vec<Cut<'_>> {
    match part.definition {
        Definition::Contract => part
            .value
            .get(keys::SCHEMA)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .filter_map(|(index, object)| {
                list_cut(
                    object,
                    format!("/{}/{index}/{}", keys::SCHEMA, keys::PROPERTIES),
                )
            })
            .collect(),
        Definition::SchemaProperty | Definition::SchemaItemProperty => {
            let properties = list_cut(part.value, format!("/{}", keys::PROPERTIES));
            let items = single_cut(part.value.get(keys::ITEMS), format!("/{}", keys::ITEMS));
            let map_entries = MAP_ENTRY_KEYS.iter().filter_map(|entry| {
                single_cut(
                    part.value.get(MAP_KEY).and_then(|map| map.get(entry)),
                    format!("/{MAP_KEY}/{entry}"),
                )
            });
            properties
                .into_iter()
                .chain(items)
                .chain(map_entries)
                .collect()
        }
    }
}

fn list_cut(parent: &Value, pointer: String) -> Option<Cut<'_>> {
    parent
        .get(keys::PROPERTIES)
        .filter(|value| value.is_array())
        .map(|value| Cut {
            pointer,
            shape: Shape::List,
            value,
        })
}

fn single_cut(value: Option<&Value>, pointer: String) -> Option<Cut<'_>> {
    value.filter(|value| value.is_object()).map(|value| Cut {
        pointer,
        shape: Shape::Single,
        value,
    })
}

/// The part with every cut replaced by its stand-in.
pub fn with_stand_ins(value: &Value, cuts: &[Cut<'_>]) -> Value {
    replaced(value, "", cuts, Shape::stand_in)
}

/// The part with every cut replaced by a value that fails validation if it is evaluated.
pub fn with_probes(value: &Value, cuts: &[Cut<'_>]) -> Value {
    replaced(value, "", cuts, Shape::probe)
}

/// Only the path down to each cut is rebuilt; everything else is cloned as is.
fn replaced(
    value: &Value,
    pointer: &str,
    cuts: &[Cut<'_>],
    replacement: fn(Shape) -> Value,
) -> Value {
    cuts.iter().find(|cut| cut.pointer == pointer).map_or_else(
        || {
            if cuts.iter().any(|cut| is_within(&cut.pointer, pointer)) {
                rebuilt(value, pointer, cuts, replacement)
            } else {
                value.clone()
            }
        },
        |cut| replacement(cut.shape),
    )
}

fn rebuilt(
    value: &Value,
    pointer: &str,
    cuts: &[Cut<'_>],
    replacement: fn(Shape) -> Value,
) -> Value {
    match value {
        Value::Object(members) => Value::Object(
            members
                .iter()
                .map(|(key, member)| {
                    let child = format!("{pointer}/{}", escape(key));
                    (key.clone(), replaced(member, &child, cuts, replacement))
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    replaced(item, &format!("{pointer}/{index}"), cuts, replacement)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn pointers(cuts: &[Cut<'_>]) -> Vec<String> {
        cuts.iter().map(|cut| cut.pointer.clone()).collect()
    }

    #[test]
    fn contract_is_cut_at_each_schema_objects_properties() {
        let value = json!({"schema": [{"name": "a", "properties": []}, {"name": "b"}]});
        let part = Part {
            value: &value,
            definition: Definition::Contract,
        };
        assert_eq!(pointers(&cuts(part)), vec!["/schema/0/properties"]);
    }

    #[test]
    fn property_is_cut_at_properties_items_and_map_entries() {
        let value = json!({
            "properties": [{"name": "a"}],
            "items": {"logicalType": "string"},
            "map": {"key": {}, "value": {}},
        });
        let part = Part {
            value: &value,
            definition: Definition::SchemaProperty,
        };
        assert_eq!(
            pointers(&cuts(part)),
            vec!["/properties", "/items", "/map/key", "/map/value"]
        );
    }

    #[test]
    fn scalars_at_an_edge_are_not_cut() {
        let value = json!({"properties": "oops", "items": 3});
        let part = Part {
            value: &value,
            definition: Definition::SchemaProperty,
        };
        assert!(cuts(part).is_empty());
    }

    #[test]
    fn stand_ins_and_probes_replace_only_the_cuts() {
        let value = json!({"name": "a", "properties": [{"name": "b"}], "items": {"name": "c"}});
        let part = Part {
            value: &value,
            definition: Definition::SchemaProperty,
        };
        let cuts = cuts(part);
        assert_eq!(
            with_stand_ins(&value, &cuts),
            json!({"name": "a", "properties": [], "items": {}})
        );
        assert_eq!(
            with_probes(&value, &cuts),
            json!({"name": "a", "properties": [null], "items": null})
        );
    }

    #[test]
    fn children_carry_pointers_and_definitions() {
        let value = json!({"properties": [{"name": "a"}, {"name": "b"}], "items": {"name": "c"}});
        let part = Part {
            value: &value,
            definition: Definition::SchemaProperty,
        };
        let children: Vec<(String, Definition)> = cuts(part)
            .iter()
            .flat_map(Cut::children)
            .map(|(pointer, child)| (pointer, child.definition))
            .collect();
        assert_eq!(
            children,
            vec![
                ("/properties/0".to_string(), Definition::SchemaProperty),
                ("/properties/1".to_string(), Definition::SchemaProperty),
                ("/items".to_string(), Definition::SchemaItemProperty),
            ]
        );
    }
}
