use super::node::{as_seq, entry};
use super::Node;

#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub node: &'a Node<'a>,
    pub key: Option<&'a Node<'a>>,
}

impl<'a> Target<'a> {
    pub fn anchor(&self) -> &'a Node<'a> {
        self.key.unwrap_or(self.node)
    }
}

/// Stops at the deepest existing node, so a pointer to a missing child points at its parent.
pub fn resolve<'a>(root: &'a Node<'a>, pointer: &str) -> Target<'a> {
    segments(pointer).fold(
        Target {
            node: root,
            key: None,
        },
        |target, segment| step(target.node, &segment).unwrap_or(target),
    )
}

pub fn resolve_exact<'a>(root: &'a Node<'a>, pointer: &str) -> Option<Target<'a>> {
    segments(pointer).try_fold(
        Target {
            node: root,
            key: None,
        },
        |target, segment| step(target.node, &segment),
    )
}

fn step<'a>(node: &'a Node<'a>, segment: &str) -> Option<Target<'a>> {
    entry(node, segment)
        .map(|(key, value)| Target {
            node: value,
            key: Some(key),
        })
        .or_else(|| {
            let index = segment.parse::<usize>().ok()?;
            as_seq(node)?.get(index).map(|item| Target {
                node: item,
                key: None,
            })
        })
}

fn segments(pointer: &str) -> impl Iterator<Item = String> + '_ {
    pointer
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Location;
    use crate::document::Document;

    const SOURCE: &str = "schema:\n  - name: tbl\n    properties:\n      - name: id\n";

    #[test]
    fn resolves_to_key_of_nested_value() {
        let document = Document::parse(SOURCE).unwrap();
        let target = resolve(document.root(), "/schema/0/properties/0/name");
        assert_eq!(
            document.location(target.anchor()),
            Location { line: 4, column: 9 }
        );
    }

    #[test]
    fn stops_at_deepest_existing_node() {
        let document = Document::parse(SOURCE).unwrap();
        let target = resolve(document.root(), "/schema/0/missing/3");
        assert_eq!(
            document.location(target.node),
            Location { line: 2, column: 5 }
        );
    }

    #[test]
    fn exact_resolution_fails_for_missing_nodes() {
        let document = Document::parse(SOURCE).unwrap();
        assert!(resolve_exact(document.root(), "/schema/0/missing").is_none());
        assert!(resolve_exact(document.root(), "/schema/0/properties/0/name").is_some());
    }

    #[test]
    fn empty_pointer_is_root() {
        let document = Document::parse(SOURCE).unwrap();
        let target = resolve(document.root(), "");
        assert!(target.key.is_none());
        assert_eq!(document.location(target.node), Location::START);
    }

    #[test]
    fn unescapes_segments() {
        let document = Document::parse("a/b: 1\n").unwrap();
        let target = resolve(document.root(), "/a~1b");
        assert!(target.key.is_some());
    }
}
