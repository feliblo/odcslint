use std::borrow::Cow;

use saphyr::{Scalar, YamlData};
use serde_json::{Map, Number, Value};

use super::node::scalar_text;
use super::Node;

pub fn to_json(node: &Node<'_>) -> Value {
    match &node.data {
        YamlData::Mapping(mapping) => Value::Object(
            mapping
                .iter()
                .map(|(key, value)| (key_text(key), to_json(value)))
                .collect::<Map<_, _>>(),
        ),
        YamlData::Sequence(items) => Value::Array(items.iter().map(to_json).collect()),
        YamlData::Value(Scalar::Boolean(value)) => Value::Bool(*value),
        YamlData::Value(Scalar::Integer(value)) => Value::from(*value),
        YamlData::Value(Scalar::FloatingPoint(value)) => {
            Number::from_f64(value.0).map_or(Value::Null, Value::Number)
        }
        YamlData::Value(Scalar::String(text)) => Value::String(text.to_string()),
        YamlData::Tagged(_, inner) => to_json(inner),
        YamlData::Value(Scalar::Null)
        | YamlData::Representation(..)
        | YamlData::Alias(_)
        | YamlData::BadValue => Value::Null,
    }
}

fn key_text(key: &Node<'_>) -> String {
    scalar_text(key).map_or_else(|| "null".to_string(), Cow::into_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::document::Document;

    #[test]
    fn converts_nested_structures() {
        let document = Document::parse("a: 1\nb: [x, true, 1.5, null]\nc:\n  d: text\n").unwrap();
        assert_eq!(
            to_json(document.root()),
            json!({"a": 1, "b": ["x", true, 1.5, null], "c": {"d": "text"}})
        );
    }

    #[test]
    fn keeps_unquoted_dates_as_strings() {
        let document = Document::parse("dateIn: 2024-01-01\n").unwrap();
        assert_eq!(to_json(document.root()), json!({"dateIn": "2024-01-01"}));
    }

    #[test]
    fn stringifies_non_string_keys() {
        let document = Document::parse("1: one\n").unwrap();
        assert_eq!(to_json(document.root()), json!({"1": "one"}));
    }

    #[test]
    fn resolves_aliases() {
        let document = Document::parse("a: &anchor {k: v}\nb: *anchor\n").unwrap();
        assert_eq!(
            to_json(document.root()),
            json!({"a": {"k": "v"}, "b": {"k": "v"}})
        );
    }
}
