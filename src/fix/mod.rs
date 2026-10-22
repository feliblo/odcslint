//! Fixes never re-serialize YAML: they replace byte ranges, so everything outside an edit is preserved.

pub mod block_seq;
pub mod text;

use std::ops::Range;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permutation {
    pub pointer: String,
    /// `order[i]` is the original index of the item placed at position `i`.
    pub order: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub edit: TextEdit,
    pub permutation: Permutation,
}

/// Overlapping fixes (e.g. a nested list inside a reordered parent) are picked up in a later pass.
pub fn non_overlapping(fixes: Vec<Fix>) -> Vec<Fix> {
    non_overlapping_by(fixes, |fix| &fix.edit)
}

pub fn non_overlapping_by<T>(items: Vec<T>, edit: impl Fn(&T) -> &TextEdit) -> Vec<T> {
    let mut sorted = items;
    sorted.sort_by_key(|item| (edit(item).range.start, edit(item).range.end));
    sorted
        .into_iter()
        .fold(Vec::new(), |mut accepted: Vec<T>, item| {
            let free = accepted
                .last()
                .is_none_or(|last| edit(last).range.end <= edit(&item).range.start);
            if free {
                accepted.push(item);
            }
            accepted
        })
}

pub fn apply(source: &str, fixes: &[Fix]) -> String {
    apply_edits(source, fixes.iter().map(|fix| &fix.edit))
}

/// Applies edits that are sorted by position and don't overlap.
pub fn apply_edits<'a>(source: &str, edits: impl IntoIterator<Item = &'a TextEdit>) -> String {
    let (mut output, end) = edits.into_iter().fold(
        (String::with_capacity(source.len()), 0),
        |(mut output, cursor), edit| {
            output.push_str(&source[cursor..edit.range.start]);
            output.push_str(&edit.replacement);
            (output, edit.range.end)
        },
    );
    output.push_str(&source[end..]);
    output
}

pub fn permuted(before: &Value, permutations: &[&Permutation]) -> Option<Value> {
    permutations
        .iter()
        .try_fold(before.clone(), |mut value, permutation| {
            let items = value.pointer_mut(&permutation.pointer)?.as_array_mut()?;
            if items.len() != permutation.order.len() {
                return None;
            }
            let reordered = permutation
                .order
                .iter()
                .map(|&index| items.get(index).cloned())
                .collect::<Option<Vec<_>>>()?;
            *items = reordered;
            Some(value)
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn fix(range: Range<usize>, replacement: &str) -> Fix {
        Fix {
            edit: TextEdit {
                range,
                replacement: replacement.to_string(),
            },
            permutation: Permutation {
                pointer: String::new(),
                order: vec![],
            },
        }
    }

    #[test]
    fn applies_edits_in_order() {
        let fixes = vec![fix(0..1, "A"), fix(2..3, "C")];
        assert_eq!(apply("abcd", &fixes), "AbCd");
    }

    #[test]
    fn drops_overlapping_fixes() {
        let accepted = non_overlapping(vec![fix(2..4, "x"), fix(0..10, "y"), fix(10..12, "z")]);
        let ranges: Vec<_> = accepted.iter().map(|fix| fix.edit.range.clone()).collect();
        assert_eq!(ranges, vec![0..10, 10..12]);
    }

    #[test]
    fn permutes_arrays_by_pointer() {
        let before = json!({"a": [{"n": 1}, {"n": 2}, {"n": 3}]});
        let permutation = Permutation {
            pointer: "/a".to_string(),
            order: vec![2, 0, 1],
        };
        assert_eq!(
            permuted(&before, &[&permutation]),
            Some(json!({"a": [{"n": 3}, {"n": 1}, {"n": 2}]}))
        );
    }

    #[test]
    fn permutation_with_wrong_length_fails() {
        let before = json!({"a": [1, 2]});
        let permutation = Permutation {
            pointer: "/a".to_string(),
            order: vec![0],
        };
        assert_eq!(permuted(&before, &[&permutation]), None);
    }
}
