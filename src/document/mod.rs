//! The only module that touches `saphyr` types directly.

pub mod node;
pub mod pointer;
pub mod to_json;
pub mod view;

use saphyr::{LoadableYamlNode, MarkedYaml, Marker};

use crate::diagnostic::Location;

pub type Node<'a> = MarkedYaml<'a>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub message: String,
    pub location: Location,
}

#[derive(Debug)]
pub struct Document<'a> {
    source: &'a str,
    root: Node<'a>,
    line_starts: Vec<usize>,
}

impl<'a> Document<'a> {
    pub fn parse(source: &'a str) -> Result<Self, SyntaxError> {
        let documents = MarkedYaml::load_from_str(source).map_err(|error| SyntaxError {
            message: error.info().to_string(),
            location: location_of(*error.marker()),
        })?;
        let mut documents = documents.into_iter();
        match (documents.next(), documents.next()) {
            (Some(root), None) => Ok(Self {
                source,
                root,
                line_starts: line_starts(source),
            }),
            (None, _) => Err(SyntaxError {
                message: "file contains no YAML document".to_string(),
                location: Location::START,
            }),
            (Some(_), Some(second)) => Err(SyntaxError {
                message: "file contains more than one YAML document".to_string(),
                location: location_of(second.span.start),
            }),
        }
    }

    pub const fn source(&self) -> &'a str {
        self.source
    }

    pub const fn root(&self) -> &Node<'a> {
        &self.root
    }

    pub fn location(&self, node: &Node<'_>) -> Location {
        location_of(node.span.start)
    }

    /// The parser counts characters, not bytes, so offsets are recomputed from the line table.
    pub fn byte_offset(&self, marker: Marker) -> usize {
        let Some(&line_start) = marker
            .line()
            .checked_sub(1)
            .and_then(|line| self.line_starts.get(line))
        else {
            return self.source.len();
        };
        self.source[line_start..]
            .char_indices()
            .nth(marker.col())
            .map_or(self.source.len(), |(offset, _)| line_start + offset)
    }
}

fn location_of(marker: Marker) -> Location {
    Location {
        line: marker.line(),
        column: marker.col() + 1,
    }
}

fn line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(source.match_indices('\n').map(|(index, _)| index + 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_document() {
        let document = Document::parse("a: 1\n").unwrap();
        assert!(node::get(document.root(), "a").is_some());
    }

    #[test]
    fn rejects_empty_file() {
        let error = Document::parse("").unwrap_err();
        assert_eq!(error.message, "file contains no YAML document");
        assert_eq!(error.location, Location::START);
    }

    #[test]
    fn rejects_multiple_documents() {
        let error = Document::parse("a: 1\n---\nb: 2\n").unwrap_err();
        assert_eq!(error.message, "file contains more than one YAML document");
        assert_eq!(error.location.line, 3);
    }

    #[test]
    fn rejects_duplicate_keys_with_location() {
        let error = Document::parse("a: 1\na: 2\n").unwrap_err();
        assert_eq!(error.location, Location { line: 2, column: 1 });
    }

    #[test]
    fn reports_syntax_error_location() {
        let error = Document::parse("a: 1\n- b\n").unwrap_err();
        assert_eq!(error.location.line, 2);
    }

    #[test]
    fn location_is_one_based() {
        let document = Document::parse("a:\n  b: 1\n").unwrap();
        let a = node::get(document.root(), "a").unwrap();
        let b = node::get(a, "b").unwrap();
        assert_eq!(document.location(b), Location { line: 2, column: 6 });
    }

    #[test]
    fn byte_offset_handles_multibyte_characters() {
        let source = "name: é😀x\nnext: value\n";
        let document = Document::parse(source).unwrap();
        let next = node::get(document.root(), "next").unwrap();
        let offset = document.byte_offset(next.span.start);
        assert_eq!(&source[offset..offset + 5], "value");
    }

    #[test]
    fn byte_offset_handles_multibyte_on_same_line() {
        let source = "é: 😀value\n";
        let document = Document::parse(source).unwrap();
        let value = node::get(document.root(), "é").unwrap();
        let offset = document.byte_offset(value.span.start);
        assert_eq!(&source[offset..], "😀value\n");
    }

    #[test]
    fn byte_offset_handles_crlf() {
        let source = "a: 1\r\nb: two\r\n";
        let document = Document::parse(source).unwrap();
        let b = node::get(document.root(), "b").unwrap();
        let offset = document.byte_offset(b.span.start);
        assert_eq!(&source[offset..offset + 3], "two");
    }
}
