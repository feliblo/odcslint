//! Operations on a contract's JSON form.
//!
//! Each operation can apply itself to the JSON (the specification) and produce the matching text
//! edit (the implementation); the upgrade checks that both agree.

use serde_json::{Map, Value};

use crate::document::pointer::resolve_exact;
use crate::document::Document;
use crate::fix::text;
use crate::fix::TextEdit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Set {
        pointer: String,
        value: Value,
    },
    Remove {
        object: String,
        key: String,
    },
    Rename {
        object: String,
        from: String,
        to: String,
    },
    Insert {
        object: String,
        key: String,
        value: Value,
    },
    Wrap {
        pointer: String,
        key: String,
    },
    Append {
        array: String,
        value: Value,
    },
}

/// The JSON pointer of `segment` inside the value at `pointer`.
pub fn child(pointer: &str, segment: &str) -> String {
    format!(
        "{pointer}/{}",
        segment.replace('~', "~0").replace('/', "~1")
    )
}

impl Op {
    /// Where the change happens, for messages.
    pub fn pointer(&self) -> &str {
        match self {
            Self::Set { pointer, .. } | Self::Wrap { pointer, .. } => pointer,
            Self::Remove { object, .. }
            | Self::Rename { object, .. }
            | Self::Insert { object, .. } => object,
            Self::Append { array, .. } => array,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Set { pointer, value } => format!("set `{pointer}` to `{value}`"),
            Self::Remove { object, key } => format!("remove `{key}` from `{object}`"),
            Self::Rename { object, from, to } => format!("rename `{from}` to `{to}` in `{object}`"),
            Self::Insert { object, key, .. } => format!("add `{key}` to `{object}`"),
            Self::Wrap { pointer, key } => format!("move `{pointer}` under `{key}`"),
            Self::Append { array, .. } => format!("add an item to `{array}`"),
        }
    }

    pub fn apply(&self, json: &mut Value) -> Option<()> {
        match self {
            Self::Set { pointer, value } => {
                *json.pointer_mut(pointer)? = value.clone();
            }
            Self::Remove { object, key } => {
                json.pointer_mut(object)?.as_object_mut()?.remove(key)?;
            }
            Self::Rename { object, from, to } => {
                let fields = json.pointer_mut(object)?.as_object_mut()?;
                if fields.contains_key(to) {
                    return None;
                }
                let value = fields.remove(from)?;
                fields.insert(to.clone(), value);
            }
            Self::Insert { object, key, value } => {
                let fields = json.pointer_mut(object)?.as_object_mut()?;
                if fields.contains_key(key) {
                    return None;
                }
                fields.insert(key.clone(), value.clone());
            }
            Self::Wrap { pointer, key } => {
                let slot = json.pointer_mut(pointer)?;
                let inner = slot.take();
                *slot = Value::Object(Map::from_iter([(key.clone(), inner)]));
            }
            Self::Append { array, value } => {
                json.pointer_mut(array)?.as_array_mut()?.push(value.clone());
            }
        }
        Some(())
    }

    /// The text edit, or `None` when this spot can't be edited safely (e.g. flow-style YAML).
    pub fn edit(&self, document: &Document<'_>) -> Option<TextEdit> {
        let root = document.root();
        match self {
            Self::Set { pointer, value } => {
                text::replace_scalar(document, resolve_exact(root, pointer)?.node, value)
            }
            Self::Remove { object, key } => {
                text::remove_entry(document, resolve_exact(root, object)?.node, key)
            }
            Self::Rename { object, from, to } => text::rename_key(
                document,
                resolve_exact(root, &child(object, from))?.key?,
                to,
            ),
            Self::Insert { object, key, value } => {
                text::insert_entry(document, resolve_exact(root, object)?.node, key, value)
            }
            Self::Wrap { pointer, key } => {
                let target = resolve_exact(root, pointer)?;
                text::wrap_value(document, target.key?, target.node, key)
            }
            Self::Append { array, value } => {
                text::append_item(document, resolve_exact(root, array)?.node, value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::document::to_json::to_json;

    fn edited(source: &str, op: &Op) -> String {
        let document = Document::parse(source).unwrap();
        let edit = op.edit(&document).unwrap();
        crate::fix::apply_edits(source, [&edit])
    }

    fn assert_text_matches_json(source: &str, op: &Op) {
        let document = Document::parse(source).unwrap();
        let mut expected = to_json(document.root());
        op.apply(&mut expected).unwrap();
        let text = edited(source, op);
        let actual = to_json(Document::parse(&text).unwrap().root());
        assert_eq!(actual, expected, "edited text:\n{text}");
    }

    #[test]
    fn every_operation_edits_text_and_json_alike() {
        let source = "apiVersion: v3.0.2\nteam:\n  - username: a\nservers:\n  - staging_dir: x\n    type: athena\nlist:\n  - property: p\n    value: 1\n";
        let ops = [
            Op::Set {
                pointer: "/apiVersion".into(),
                value: json!("v3.1.0"),
            },
            Op::Remove {
                object: "/servers/0".into(),
                key: "staging_dir".into(),
            },
            Op::Rename {
                object: "/servers/0".into(),
                from: "staging_dir".into(),
                to: "stagingDir".into(),
            },
            Op::Insert {
                object: "/servers/0".into(),
                key: "schema".into(),
                value: json!("db"),
            },
            Op::Wrap {
                pointer: "/team".into(),
                key: "members".into(),
            },
            Op::Append {
                array: "/list".into(),
                value: json!({"property": "q", "value": null}),
            },
        ];
        for op in &ops {
            assert_text_matches_json(source, op);
        }
    }

    #[test]
    fn child_escapes_pointer_segments() {
        assert_eq!(child("/a", "b/c~d"), "/a/b~1c~0d");
    }

    #[test]
    fn insert_refuses_existing_keys() {
        let mut json = json!({"a": 1});
        let op = Op::Insert {
            object: String::new(),
            key: "a".into(),
            value: json!(2),
        };
        assert!(op.apply(&mut json).is_none());
    }
}
