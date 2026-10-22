//! Formatting-preserving edits on block-style YAML. Extents come from indentation because the
//! parser's collection end markers are unreliable.

use serde_json::Value;

use super::TextEdit;
use crate::document::node::{as_seq, entries, scalar_text};
use crate::document::{Document, Node};

pub const COMMENT: char = '#';
pub const DASH: char = '-';
pub const CRLF: &str = "\r\n";
pub const LF: &str = "\n";
const FLOW_STARTS: [char; 2] = ['[', '{'];
const BLOCK_SCALAR_STARTS: [char; 2] = ['|', '>'];
const QUOTES: [char; 2] = ['"', '\''];
const INDENT_STEP: usize = 2;
const RESERVED_WORDS: [&str; 10] = [
    "true", "false", "null", "~", "yes", "no", "on", "off", "y", "n",
];

pub fn line_start(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |index| index + 1)
}

pub fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

pub fn line_ending(source: &str) -> &'static str {
    if source.contains(CRLF) {
        CRLF
    } else {
        LF
    }
}

pub fn dash_before(source: &str, item_start: usize) -> Option<usize> {
    let before = source[..item_start].trim_end();
    before.ends_with(DASH).then(|| before.len() - 1)
}

pub fn dash_indent(source: &str, dash: usize) -> Option<usize> {
    let line = line_start(source, dash);
    source[line..dash]
        .chars()
        .all(|character| character == ' ')
        .then_some(dash - line)
}

pub fn comments_above(source: &str, line: usize, indent: usize) -> usize {
    std::iter::successors(Some(line), |&current| {
        let previous = line_start(source, current.checked_sub(1)?);
        let text = source[previous..current].trim_end();
        (text.trim_start().starts_with(COMMENT) && indent_of(text) == indent).then_some(previous)
    })
    .last()
    .unwrap_or(line)
}

/// The end of the block that starts on the line at `first_line`: after the last non-blank line
/// indented deeper than `indent` (or, for an unindented sequence, a `-` line at `indent`).
pub fn block_end(source: &str, first_line: usize, indent: usize, sequence_value: bool) -> usize {
    let mut lines = source[first_line..]
        .split_inclusive('\n')
        .scan(first_line, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line))
        });
    let first_end = lines
        .next()
        .map_or(first_line, |(start, line)| start + line.len());
    lines
        .take_while(|(_, line)| {
            let unindented_item =
                sequence_value && indent_of(line) == indent && line.trim_start().starts_with(DASH);
            line.trim().is_empty() || indent_of(line) > indent || unindented_item
        })
        .filter(|(_, line)| !line.trim().is_empty())
        .last()
        .map_or(first_end, |(start, line)| start + line.len())
}

fn is_flow(document: &Document<'_>, node: &Node<'_>) -> bool {
    document.source()[document.byte_offset(node.span.start)..].starts_with(FLOW_STARTS)
}

fn key_column(document: &Document<'_>, key: &Node<'_>) -> usize {
    let start = document.byte_offset(key.span.start);
    start - line_start(document.source(), start)
}

fn starts_its_line(document: &Document<'_>, key: &Node<'_>) -> bool {
    let start = document.byte_offset(key.span.start);
    document.source()[line_start(document.source(), start)..start]
        .chars()
        .all(|character| character == ' ')
}

fn entry_end(document: &Document<'_>, key: &Node<'_>, value: &Node<'_>) -> usize {
    let source = document.source();
    let line = line_start(source, document.byte_offset(key.span.start));
    let indent = indent_of(&source[line..]);
    let column = key_column(document, key);
    block_end(source, line, column.max(indent), as_seq(value).is_some())
}

pub fn replace_scalar(document: &Document<'_>, node: &Node<'_>, value: &Value) -> Option<TextEdit> {
    let source = document.source();
    let start = document.byte_offset(node.span.start);
    let end = document.byte_offset(node.span.end);
    let original = source.get(start..end)?;
    if original.starts_with(BLOCK_SCALAR_STARTS) || original.starts_with(FLOW_STARTS) {
        return None;
    }
    let replacement = match (original.chars().next(), value) {
        (Some('"'), Value::String(text)) => serde_json::to_string(text).ok()?,
        (Some('\''), Value::String(text)) => format!("'{}'", text.replace('\'', "''")),
        _ => scalar(value)?,
    };
    Some(TextEdit {
        range: start..end,
        replacement,
    })
}

pub fn rename_key(document: &Document<'_>, key: &Node<'_>, new_key: &str) -> Option<TextEdit> {
    let start = document.byte_offset(key.span.start);
    let end = document.byte_offset(key.span.end);
    if document.source().get(start..end)?.starts_with(QUOTES) {
        return None;
    }
    Some(TextEdit {
        range: start..end,
        replacement: render_key(new_key),
    })
}

/// Removes `key` and its value from a block mapping. A key that shares its line with the `-` of a
/// sequence item hands that line to the next key; removing the only key of an item is not supported.
pub fn remove_entry(document: &Document<'_>, mapping: &Node<'_>, key: &str) -> Option<TextEdit> {
    if is_flow(document, mapping) {
        return None;
    }
    let pairs: Vec<(&Node<'_>, &Node<'_>)> = entries(mapping).collect();
    let position = pairs
        .iter()
        .position(|(candidate, _)| scalar_text(candidate).as_deref() == Some(key))?;
    let (key_node, value) = pairs[position];
    let source = document.source();
    let key_start = document.byte_offset(key_node.span.start);
    if starts_its_line(document, key_node) {
        let start = line_start(source, key_start);
        Some(TextEdit {
            range: start..entry_end(document, key_node, value),
            replacement: String::new(),
        })
    } else {
        let (next_key, _) = pairs.get(position + 1)?;
        Some(TextEdit {
            range: key_start..document.byte_offset(next_key.span.start),
            replacement: String::new(),
        })
    }
}

/// Appends `key: value` after the last entry of a non-empty block mapping.
pub fn insert_entry(
    document: &Document<'_>,
    mapping: &Node<'_>,
    key: &str,
    value: &Value,
) -> Option<TextEdit> {
    if is_flow(document, mapping) {
        return None;
    }
    let pairs: Vec<(&Node<'_>, &Node<'_>)> = entries(mapping).collect();
    let (first_key, _) = pairs.first()?;
    let (last_key, last_value) = pairs.last()?;
    let source = document.source();
    let eol = line_ending(source);
    let position = entry_end(document, last_key, last_value);
    let lead = if source[..position].ends_with(LF) {
        ""
    } else {
        eol
    };
    Some(TextEdit {
        range: position..position,
        replacement: format!(
            "{lead}{}",
            render_entry(key, value, key_column(document, first_key), eol)
        ),
    })
}

/// Turns `key:` followed by a block value into `key:` / `  wrapper:` with the value indented below it.
pub fn wrap_value(
    document: &Document<'_>,
    key: &Node<'_>,
    value: &Node<'_>,
    wrapper: &str,
) -> Option<TextEdit> {
    if is_flow(document, value) || !starts_its_line(document, key) {
        return None;
    }
    let source = document.source();
    let key_line = line_start(source, document.byte_offset(key.span.start));
    let value_line = line_start(source, document.byte_offset(value.span.start));
    if value_line == key_line {
        return None;
    }
    let body_start = key_line + source[key_line..].find('\n')? + 1;
    let end = entry_end(document, key, value);
    let eol = line_ending(source);
    let column = key_column(document, key);
    let indented: String = source[body_start..end]
        .split_inclusive('\n')
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("{}{line}", " ".repeat(INDENT_STEP))
            }
        })
        .collect();
    Some(TextEdit {
        range: body_start..end,
        replacement: format!(
            "{}{}:{eol}{indented}",
            " ".repeat(column + INDENT_STEP),
            render_key(wrapper)
        ),
    })
}

/// Appends an item to a non-empty block sequence, at the indentation of its other items.
pub fn append_item(
    document: &Document<'_>,
    sequence: &Node<'_>,
    value: &Value,
) -> Option<TextEdit> {
    if is_flow(document, sequence) {
        return None;
    }
    let items = as_seq(sequence)?;
    let last = items.last()?;
    let source = document.source();
    let dash = dash_before(source, document.byte_offset(last.span.start))?;
    let indent = dash_indent(source, dash)?;
    let position = block_end(source, line_start(source, dash), indent, false);
    let eol = line_ending(source);
    let lead = if source[..position].ends_with(LF) {
        ""
    } else {
        eol
    };
    Some(TextEdit {
        range: position..position,
        replacement: format!("{lead}{}", render_item(value, indent, eol)),
    })
}

/// A YAML scalar for `value`, quoted only when a plain scalar would be read differently.
pub fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("null".to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) if is_plain_safe(text) => Some(text.clone()),
        Value::String(text) => serde_json::to_string(text).ok(),
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn is_plain_safe(text: &str) -> bool {
    let allowed = text
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "_-./@".contains(character));
    let starts_safely = text
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '/');
    let reserved = RESERVED_WORDS.contains(&text.to_ascii_lowercase().as_str());
    allowed && starts_safely && !reserved
}

fn render_key(key: &str) -> String {
    scalar(&Value::String(key.to_string())).unwrap_or_else(|| key.to_string())
}

fn render_inline(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) if items.is_empty() => Some("[]".to_string()),
        Value::Object(fields) if fields.is_empty() => Some("{}".to_string()),
        Value::Array(items) => items
            .iter()
            .map(scalar)
            .collect::<Option<Vec<_>>>()
            .map(|items| format!("[{}]", items.join(", "))),
        other => scalar(other),
    }
}

fn render_entry(key: &str, value: &Value, indent: usize, eol: &str) -> String {
    let pad = " ".repeat(indent);
    let key = render_key(key);
    render_inline(value).map_or_else(
        || {
            let body = render_block(value, indent + INDENT_STEP, eol);
            format!("{pad}{key}:{eol}{body}")
        },
        |inline| format!("{pad}{key}: {inline}{eol}"),
    )
}

fn render_block(value: &Value, indent: usize, eol: &str) -> String {
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| render_entry(key, value, indent, eol))
            .collect(),
        Value::Array(items) => items
            .iter()
            .map(|item| render_item(item, indent, eol))
            .collect(),
        scalar_value => format!(
            "{}{}{eol}",
            " ".repeat(indent),
            scalar(scalar_value).unwrap_or_default()
        ),
    }
}

fn render_item(value: &Value, indent: usize, eol: &str) -> String {
    let pad = " ".repeat(indent);
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            let body = render_block(value, indent + INDENT_STEP, eol);
            let trimmed = body.trim_start_matches(' ');
            format!("{pad}{DASH} {trimmed}")
        }
        other => format!(
            "{pad}{DASH} {}{eol}",
            render_inline(other).unwrap_or_default()
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::document::node::get;

    fn apply_edit(source: &str, edit: &TextEdit) -> String {
        format!(
            "{}{}{}",
            &source[..edit.range.start],
            edit.replacement,
            &source[edit.range.end..]
        )
    }

    #[test]
    fn replaces_scalars_and_keeps_quote_style() {
        let source = "a: \"v3.0.0\"\nb: v3.1.0 # keep\nc: 1\n";
        let document = Document::parse(source).unwrap();
        let root = document.root();
        let quoted = replace_scalar(&document, get(root, "a").unwrap(), &json!("v3.2.0")).unwrap();
        let plain = replace_scalar(&document, get(root, "b").unwrap(), &json!("v3.2.0")).unwrap();
        let number = replace_scalar(&document, get(root, "c").unwrap(), &json!("1")).unwrap();
        assert_eq!(
            apply_edit(source, &quoted),
            "a: \"v3.2.0\"\nb: v3.1.0 # keep\nc: 1\n"
        );
        assert_eq!(
            apply_edit(source, &plain),
            "a: \"v3.0.0\"\nb: v3.2.0 # keep\nc: 1\n"
        );
        assert_eq!(
            apply_edit(source, &number),
            "a: \"v3.0.0\"\nb: v3.1.0 # keep\nc: \"1\"\n"
        );
    }

    #[test]
    fn removes_entries_with_nested_blocks() {
        let source = "a: 1\nb:\n  c: 2\n  d: [3]\ne: 4\n";
        let document = Document::parse(source).unwrap();
        let edit = remove_entry(&document, document.root(), "b").unwrap();
        assert_eq!(apply_edit(source, &edit), "a: 1\ne: 4\n");
    }

    #[test]
    fn removing_the_first_key_of_an_item_keeps_the_dash() {
        let source = "list:\n  - staging_dir: x\n    schema: y\n";
        let document = Document::parse(source).unwrap();
        let item = &as_seq(get(document.root(), "list").unwrap()).unwrap()[0];
        let edit = remove_entry(&document, item, "staging_dir").unwrap();
        assert_eq!(apply_edit(source, &edit), "list:\n  - schema: y\n");
    }

    #[test]
    fn inserts_entries_at_the_mapping_indentation() {
        let source = "list:\n  - a: 1\n    b:\n      c: 2\nnext: 3\n";
        let document = Document::parse(source).unwrap();
        let item = &as_seq(get(document.root(), "list").unwrap()).unwrap()[0];
        let edit = insert_entry(
            &document,
            item,
            "d",
            &json!([{"property": "p", "value": "v"}]),
        )
        .unwrap();
        assert_eq!(
            apply_edit(source, &edit),
            "list:\n  - a: 1\n    b:\n      c: 2\n    d:\n      - property: p\n        value: v\nnext: 3\n"
        );
    }

    #[test]
    fn wraps_a_block_value_and_keeps_comments() {
        let source =
            "team:\n  # owners\n  - username: a\n    role: owner\n  - username: b\nnext: 1\n";
        let document = Document::parse(source).unwrap();
        let (key, value) = crate::document::node::entry(document.root(), "team").unwrap();
        let edit = wrap_value(&document, key, value, "members").unwrap();
        assert_eq!(
            apply_edit(source, &edit),
            "team:\n  members:\n    # owners\n    - username: a\n      role: owner\n    - username: b\nnext: 1\n"
        );
    }

    #[test]
    fn wraps_unindented_sequences() {
        let source = "team:\n- username: a\nnext: 1\n";
        let document = Document::parse(source).unwrap();
        let (key, value) = crate::document::node::entry(document.root(), "team").unwrap();
        let edit = wrap_value(&document, key, value, "members").unwrap();
        assert_eq!(
            apply_edit(source, &edit),
            "team:\n  members:\n  - username: a\nnext: 1\n"
        );
    }

    #[test]
    fn appends_items_to_block_sequences() {
        let source = "list:\n  - property: a\n    value: 1\n";
        let document = Document::parse(source).unwrap();
        let sequence = get(document.root(), "list").unwrap();
        let edit = append_item(
            &document,
            sequence,
            &json!({"property": "b", "value": null}),
        )
        .unwrap();
        assert_eq!(
            apply_edit(source, &edit),
            "list:\n  - property: a\n    value: 1\n  - property: b\n    value: null\n"
        );
    }

    #[test]
    fn refuses_flow_collections() {
        let source = "team: [{username: a}]\nmap: {a: 1}\n";
        let document = Document::parse(source).unwrap();
        let (key, value) = crate::document::node::entry(document.root(), "team").unwrap();
        assert!(wrap_value(&document, key, value, "members").is_none());
        assert!(insert_entry(
            &document,
            get(document.root(), "map").unwrap(),
            "b",
            &json!(2)
        )
        .is_none());
    }

    #[test]
    fn quotes_strings_only_when_needed() {
        assert_eq!(scalar(&json!("orders_v1")).unwrap(), "orders_v1");
        assert_eq!(scalar(&json!("1")).unwrap(), "\"1\"");
        assert_eq!(scalar(&json!("true")).unwrap(), "\"true\"");
        assert_eq!(scalar(&json!("a: b")).unwrap(), "\"a: b\"");
    }

    #[test]
    fn keeps_crlf_and_missing_final_newline() {
        let source = "a: 1\r\nb: 2";
        let document = Document::parse(source).unwrap();
        let edit = insert_entry(&document, document.root(), "c", &json!(3)).unwrap();
        assert_eq!(apply_edit(source, &edit), "a: 1\r\nb: 2\r\nc: 3\r\n");
    }
}
