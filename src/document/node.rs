use std::borrow::Cow;

use saphyr::{Scalar, YamlData};

use super::Node;

pub fn untag<'a>(node: &'a Node<'a>) -> &'a Node<'a> {
    match &node.data {
        YamlData::Tagged(_, inner) => untag(inner),
        _ => node,
    }
}

pub fn entry<'a>(node: &'a Node<'a>, key: &str) -> Option<(&'a Node<'a>, &'a Node<'a>)> {
    let YamlData::Mapping(mapping) = &untag(node).data else {
        return None;
    };
    mapping
        .iter()
        .find(|(candidate, _)| scalar_text(candidate).as_deref() == Some(key))
        .map(|(key_node, value)| (key_node, untag(value)))
}

pub fn get<'a>(node: &'a Node<'a>, key: &str) -> Option<&'a Node<'a>> {
    entry(node, key).map(|(_, value)| value)
}

pub fn get_path<'a>(node: &'a Node<'a>, path: &str) -> Option<&'a Node<'a>> {
    path.split('.')
        .try_fold(node, |current, segment| get(current, segment))
}

pub fn entries<'a>(node: &'a Node<'a>) -> impl Iterator<Item = (&'a Node<'a>, &'a Node<'a>)> {
    let mapping = match &untag(node).data {
        YamlData::Mapping(mapping) => Some(mapping),
        _ => None,
    };
    mapping
        .into_iter()
        .flat_map(|mapping| mapping.iter().map(|(key, value)| (key, untag(value))))
}

pub fn as_seq<'a>(node: &'a Node<'a>) -> Option<&'a [Node<'a>]> {
    match &untag(node).data {
        YamlData::Sequence(items) => Some(items.as_slice()),
        _ => None,
    }
}

pub fn as_str<'a>(node: &'a Node<'a>) -> Option<&'a str> {
    match &untag(node).data {
        YamlData::Value(Scalar::String(text)) => Some(text.as_ref()),
        _ => None,
    }
}

pub fn as_bool(node: &Node<'_>) -> Option<bool> {
    match &untag(node).data {
        YamlData::Value(Scalar::Boolean(value)) => Some(*value),
        _ => None,
    }
}

pub fn as_i64(node: &Node<'_>) -> Option<i64> {
    match &untag(node).data {
        YamlData::Value(Scalar::Integer(value)) => Some(*value),
        _ => None,
    }
}

pub fn scalar_text<'a>(node: &'a Node<'a>) -> Option<Cow<'a, str>> {
    match &untag(node).data {
        YamlData::Value(Scalar::String(text)) => Some(Cow::Borrowed(text.as_ref())),
        YamlData::Value(Scalar::Integer(value)) => Some(Cow::Owned(value.to_string())),
        YamlData::Value(Scalar::FloatingPoint(value)) => Some(Cow::Owned(value.to_string())),
        YamlData::Value(Scalar::Boolean(value)) => Some(Cow::Owned(value.to_string())),
        _ => None,
    }
}

pub fn is_empty(node: &Node<'_>) -> bool {
    match &untag(node).data {
        YamlData::Value(Scalar::Null) => true,
        YamlData::Value(Scalar::String(text)) => text.trim().is_empty(),
        YamlData::Sequence(items) => items.is_empty(),
        YamlData::Mapping(mapping) => mapping.is_empty(),
        _ => false,
    }
}

pub fn scalar_items<'a>(node: &'a Node<'a>) -> Vec<&'a Node<'a>> {
    as_seq(node).map_or_else(
        || {
            scalar_text(node)
                .map(|_| vec![untag(node)])
                .unwrap_or_default()
        },
        |items| {
            items
                .iter()
                .map(untag)
                .filter(|item| scalar_text(item).is_some())
                .collect()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn get_path_follows_nested_mappings() {
        let document = Document::parse("a:\n  b:\n    c: deep\n").unwrap();
        let node = get_path(document.root(), "a.b.c").unwrap();
        assert_eq!(as_str(node), Some("deep"));
    }

    #[test]
    fn get_path_returns_none_for_missing_segment() {
        let document = Document::parse("a:\n  b: 1\n").unwrap();
        assert!(get_path(document.root(), "a.x").is_none());
    }

    #[test]
    fn scalar_text_formats_non_strings() {
        let document = Document::parse("i: 3\nb: true\n").unwrap();
        let root = document.root();
        assert_eq!(scalar_text(get(root, "i").unwrap()).as_deref(), Some("3"));
        assert_eq!(
            scalar_text(get(root, "b").unwrap()).as_deref(),
            Some("true")
        );
    }

    #[test]
    fn is_empty_detects_blank_values() {
        let document = Document::parse("a: ''\nb: []\nc: {}\nd:\ne: x\n").unwrap();
        let root = document.root();
        let empties: Vec<bool> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|key| is_empty(get(root, key).unwrap()))
            .collect();
        assert_eq!(empties, vec![true, true, true, true, false]);
    }

    #[test]
    fn scalar_items_flattens_sequences() {
        let document = Document::parse("one: a\nmany: [b, c]\n").unwrap();
        let root = document.root();
        assert_eq!(scalar_items(get(root, "one").unwrap()).len(), 1);
        assert_eq!(scalar_items(get(root, "many").unwrap()).len(), 2);
    }

    #[test]
    fn tagged_values_are_transparent() {
        let document = Document::parse("a: !custom text\n").unwrap();
        assert_eq!(as_str(get(document.root(), "a").unwrap()), Some("text"));
    }
}
