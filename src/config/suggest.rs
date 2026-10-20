const UNKNOWN_MARKER: &str = "unknown ";
const EXPECTED_MARKER: &str = "expected ";
const DID_YOU_MEAN: &str = "did you mean";

/// Appends a "did you mean" hint to serde's "unknown field/variant `x`, expected one of ..."
/// messages when one of the expected names is close to what was typed.
pub fn with_suggestion(message: &str) -> String {
    if message.contains(DID_YOU_MEAN) {
        return message.to_string();
    }
    let typed = unknown_name(message);
    let suggestion = typed.and_then(|typed| closest(typed, &expected_names(message)));
    suggestion.map_or_else(
        || message.to_string(),
        |suggestion| format!("{message}; {DID_YOU_MEAN} `{suggestion}`?"),
    )
}

/// The name serde reports as unknown, e.g. `valuse` in "unknown field `valuse`, ...".
pub fn unknown_name(message: &str) -> Option<&str> {
    let after = &message[message.find(UNKNOWN_MARKER)?..];
    after.split('`').nth(1)
}

fn expected_names(message: &str) -> Vec<&str> {
    message
        .find(EXPECTED_MARKER)
        .map(|start| message[start..].split('`').skip(1).step_by(2).collect())
        .unwrap_or_default()
}

/// The candidate within a small edit distance of `typed`, if any.
pub fn closest<'a>(typed: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let limit = (typed.chars().count() / 3).max(2);
    candidates
        .iter()
        .map(|candidate| (edit_distance(typed, candidate), *candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Looks up `name` in a table of `(ODCS path, short name, value)` entries; either spelling works.
/// Errors list the ODCS paths, since those are what users see in their contracts.
pub fn lookup<T: Clone>(name: &str, table: &[(&str, &str, T)], noun: &str) -> Result<T, String> {
    table
        .iter()
        .find(|(path, short, _)| name == *path || name == *short)
        .map(|(_, _, value)| value.clone())
        .ok_or_else(|| {
            let spellings: Vec<&str> = table
                .iter()
                .flat_map(|(path, short, _)| [*path, *short])
                .collect();
            let paths: Vec<String> = table
                .iter()
                .map(|(path, _, _)| format!("`{path}`"))
                .collect();
            let hint = closest(name, &spellings).map_or_else(
                || ".".to_string(),
                |suggestion| format!("; {DID_YOU_MEAN} `{suggestion}`?"),
            );
            format!(
                "unknown {noun} `{name}`{hint} Expected one of {}",
                paths.join(", ")
            )
        })
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let first_row: Vec<usize> = (0..=right.len()).collect();
    left.chars()
        .enumerate()
        .fold(first_row, |previous, (row, left_char)| {
            right
                .iter()
                .enumerate()
                .fold(vec![row + 1], |mut current, (column, right_char)| {
                    let substitution = previous[column] + usize::from(left_char != *right_char);
                    let value = substitution
                        .min(previous[column + 1] + 1)
                        .min(current[column] + 1);
                    current.push(value);
                    current
                })
        })
        .last()
        .copied()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_close_field_name() {
        assert_eq!(
            with_suggestion("unknown field `valuse`, expected one of `field`, `values`, `pattern`"),
            "unknown field `valuse`, expected one of `field`, `values`, `pattern`; did you mean `values`?"
        );
    }

    #[test]
    fn no_suggestion_when_nothing_is_close() {
        let message = "unknown variant `zzz`, expected one of `allowed_values`, `primary_key`";
        assert_eq!(with_suggestion(message), message);
    }

    #[test]
    fn suggests_variant_with_single_expected_name() {
        assert_eq!(
            with_suggestion("unknown variant `allowed_value`, expected `allowed_values`"),
            "unknown variant `allowed_value`, expected `allowed_values`; did you mean `allowed_values`?"
        );
    }

    #[test]
    fn lookup_accepts_both_spellings() {
        let table = [("team.name", "team_name", 1), ("domain", "domain", 2)];
        assert_eq!(lookup("team.name", &table, "field"), Ok(1));
        assert_eq!(lookup("team_name", &table, "field"), Ok(1));
    }

    #[test]
    fn lookup_error_suggests_and_lists_paths() {
        let table = [("team.name", "team_name", 1), ("domain", "domain", 2)];
        assert_eq!(
            lookup("domian", &table, "field"),
            Err("unknown field `domian`; did you mean `domain`? Expected one of `team.name`, `domain`".to_string())
        );
    }

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("same", "same"), 0);
    }
}
