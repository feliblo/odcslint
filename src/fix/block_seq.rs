//! The parser's collection end markers are unreliable, so item extents are derived from indentation.

use super::TextEdit;
use crate::document::node::as_seq;
use crate::document::{Document, Node};

const COMMENT: char = '#';
const DASH: char = '-';
const FLOW_SEQUENCE_START: char = '[';
const CRLF: &str = "\r\n";
const LF: &str = "\n";

pub fn reorder(document: &Document<'_>, sequence: &Node<'_>, order: &[usize]) -> Option<TextEdit> {
    let source = document.source();
    let items = as_seq(sequence)?;
    let sequence_start = document.byte_offset(sequence.span.start);
    if items.len() < 2
        || items.len() != order.len()
        || source[sequence_start..].starts_with(FLOW_SEQUENCE_START)
    {
        return None;
    }

    let item_starts: Vec<usize> = items
        .iter()
        .map(|item| document.byte_offset(item.span.start))
        .collect();
    let dashes = item_starts
        .iter()
        .map(|&start| dash_before(source, start))
        .collect::<Option<Vec<_>>>()?;
    let indent = dash_indent(source, dashes[0])?;
    let same_indent = dashes
        .iter()
        .all(|&dash| dash_indent(source, dash) == Some(indent));
    if !same_indent {
        return None;
    }

    let chunks: Vec<(usize, usize)> = dashes
        .iter()
        .map(|&dash| {
            let line = line_start(source, dash);
            (
                comments_above(source, line, indent),
                item_end(source, line, indent),
            )
        })
        .collect();
    if !well_formed(&chunks, &item_starts) {
        return None;
    }

    let line_ending = if source.contains(CRLF) { CRLF } else { LF };
    let separators: Vec<&str> = chunks
        .windows(2)
        .map(|pair| &source[pair[0].1..pair[1].0])
        .collect();
    let replacement = order.iter().enumerate().fold(
        String::with_capacity(source.len()),
        |mut replacement, (position, &index)| {
            let (start, end) = chunks[index];
            let chunk = &source[start..end];
            replacement.push_str(chunk);
            if !chunk.ends_with(LF) {
                replacement.push_str(line_ending);
            }
            replacement.push_str(separators.get(position).copied().unwrap_or(""));
            replacement
        },
    );

    let range = chunks[0].0..chunks[chunks.len() - 1].1;
    let replacement = if source[range.clone()].ends_with(LF) {
        replacement
    } else {
        replacement
            .strip_suffix(line_ending)
            .unwrap_or(&replacement)
            .to_string()
    };
    Some(TextEdit { range, replacement })
}

fn dash_before(source: &str, item_start: usize) -> Option<usize> {
    let before = source[..item_start].trim_end();
    before.ends_with(DASH).then(|| before.len() - 1)
}

fn dash_indent(source: &str, dash: usize) -> Option<usize> {
    let line = line_start(source, dash);
    source[line..dash]
        .chars()
        .all(|character| character == ' ')
        .then_some(dash - line)
}

fn line_start(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |index| index + 1)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn comments_above(source: &str, line: usize, indent: usize) -> usize {
    std::iter::successors(Some(line), |&current| {
        let previous = line_start(source, current.checked_sub(1)?);
        let text = source[previous..current].trim_end();
        (text.trim_start().starts_with(COMMENT) && indent_of(text) == indent).then_some(previous)
    })
    .last()
    .unwrap_or(line)
}

fn item_end(source: &str, dash_line: usize, indent: usize) -> usize {
    let mut lines = source[dash_line..]
        .split_inclusive('\n')
        .scan(dash_line, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line))
        });
    let first_end = lines
        .next()
        .map_or(dash_line, |(start, line)| start + line.len());
    lines
        .take_while(|(_, line)| line.trim().is_empty() || indent_of(line) > indent)
        .filter(|(_, line)| !line.trim().is_empty())
        .last()
        .map_or(first_end, |(start, line)| start + line.len())
}

fn well_formed(chunks: &[(usize, usize)], item_starts: &[usize]) -> bool {
    let ordered = chunks.windows(2).all(|pair| pair[0].1 <= pair[1].0);
    let contain_items = chunks
        .iter()
        .zip(item_starts)
        .all(|(&(start, end), &item)| start <= item && item < end);
    ordered && contain_items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::node::get;

    fn reorder_key(source: &str, key: &str, order: &[usize]) -> Option<String> {
        let document = Document::parse(source).unwrap();
        let sequence = get(document.root(), key).unwrap();
        reorder(&document, sequence, order).map(|edit| {
            format!(
                "{}{}{}",
                &source[..edit.range.start],
                edit.replacement,
                &source[edit.range.end..]
            )
        })
    }

    #[test]
    fn swaps_simple_items() {
        let source = "list:\n  - name: a\n  - name: b\nafter: 1\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\n  - name: b\n  - name: a\nafter: 1\n"
        );
    }

    #[test]
    fn moves_leading_and_trailing_comments_with_items() {
        let source = "list:\n  # about a\n  - name: a # inline\n    # inner\n  - name: b\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\n  - name: b\n  # about a\n  - name: a # inline\n    # inner\n"
        );
    }

    #[test]
    fn keeps_blank_line_separators_in_place() {
        let source = "list:\n  - a\n\n  - b\n  - c\n";
        assert_eq!(
            reorder_key(source, "list", &[2, 1, 0]).unwrap(),
            "list:\n  - c\n\n  - b\n  - a\n"
        );
    }

    #[test]
    fn keeps_shallower_comment_after_list_in_place() {
        let source = "list:\n  - a\n  - b\n# end of list\nafter: 1\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\n  - b\n  - a\n# end of list\nafter: 1\n"
        );
    }

    #[test]
    fn moves_block_scalars_intact() {
        let source = "list:\n  - name: a\n    description: |\n      line one\n\n      line two\n  - name: b\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\n  - name: b\n  - name: a\n    description: |\n      line one\n\n      line two\n"
        );
    }

    #[test]
    fn supports_unindented_sequences() {
        let source = "list:\n- name: a\n  x: 1\n- name: b\nafter: 1\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\n- name: b\n- name: a\n  x: 1\nafter: 1\n"
        );
    }

    #[test]
    fn handles_missing_final_newline() {
        let source = "list:\n  - a\n  - b";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\n  - b\n  - a"
        );
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let source = "list:\r\n  - a\r\n  - b\r\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "list:\r\n  - b\r\n  - a\r\n"
        );
    }

    #[test]
    fn handles_multibyte_text_before_the_list() {
        let source = "naam: é😀\nlist:\n  - ä\n  - b\n";
        assert_eq!(
            reorder_key(source, "list", &[1, 0]).unwrap(),
            "naam: é😀\nlist:\n  - b\n  - ä\n"
        );
    }

    #[test]
    fn refuses_flow_sequences() {
        assert_eq!(reorder_key("list: [a, b]\n", "list", &[1, 0]), None);
    }

    #[test]
    fn refuses_single_item() {
        assert_eq!(reorder_key("list:\n  - a\n", "list", &[0]), None);
    }

    #[test]
    fn identity_order_reproduces_source() {
        let source = "list:\n  - a\n  # c\n  - b\n";
        assert_eq!(reorder_key(source, "list", &[0, 1]).unwrap(), source);
    }
}
