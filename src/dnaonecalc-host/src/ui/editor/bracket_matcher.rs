//! Bracket-pair matching for the formula editor.
//!
//! Finds the bracket that matches the character at or immediately before the
//! caret, taking nesting into account and ignoring brackets that fall inside
//! string literals (wrapped in `"` ... `"`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracketPairHighlight {
    pub open_offset: usize,
    pub close_offset: usize,
}

pub fn bracket_pair_for_caret(text: &str, caret_offset: usize) -> Option<BracketPairHighlight> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    // Prefer a bracket at the caret, fall back to the bracket just before it.
    let probe = bracket_at(&chars, caret_offset).or_else(|| {
        caret_offset
            .checked_sub(1)
            .and_then(|p| bracket_at(&chars, p))
    });

    let (probe_index, probe_char) = probe?;
    if is_inside_string_literal(&chars, probe_index) {
        return None;
    }
    if is_open_bracket(probe_char) {
        find_matching_close(&chars, probe_index).map(|close| BracketPairHighlight {
            open_offset: probe_index,
            close_offset: close,
        })
    } else if is_close_bracket(probe_char) {
        find_matching_open(&chars, probe_index).map(|open| BracketPairHighlight {
            open_offset: open,
            close_offset: probe_index,
        })
    } else {
        None
    }
}

fn is_inside_string_literal(chars: &[char], index: usize) -> bool {
    let mut in_string = false;
    for (idx, ch) in chars.iter().enumerate() {
        if idx >= index {
            break;
        }
        if *ch == '"' {
            in_string = !in_string;
        }
    }
    in_string
}

fn bracket_at(chars: &[char], index: usize) -> Option<(usize, char)> {
    chars
        .get(index)
        .copied()
        .filter(|ch| is_open_bracket(*ch) || is_close_bracket(*ch))
        .map(|ch| (index, ch))
}

fn is_open_bracket(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}

fn is_close_bracket(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

fn matches_pair(open: char, close: char) -> bool {
    matches!((open, close), ('(', ')') | ('[', ']') | ('{', '}'))
}

fn find_matching_close(chars: &[char], open_index: usize) -> Option<usize> {
    let open_char = chars[open_index];
    let mut depth = 0usize;
    let mut in_string = false;
    for (idx, ch) in chars.iter().enumerate().skip(open_index) {
        if in_string {
            if *ch == '"' {
                in_string = false;
            }
            continue;
        }
        if *ch == '"' {
            in_string = true;
            continue;
        }
        if is_open_bracket(*ch) {
            depth += 1;
            continue;
        }
        if is_close_bracket(*ch) {
            depth -= 1;
            if depth == 0 {
                if matches_pair(open_char, *ch) {
                    return Some(idx);
                }
                return None;
            }
        }
    }
    None
}

fn find_matching_open(chars: &[char], close_index: usize) -> Option<usize> {
    let close_char = chars[close_index];
    let mut depth = 0usize;
    let mut in_string = false;
    for idx in (0..=close_index).rev() {
        let ch = chars[idx];
        // String tracking from the right is tricky (paired quotes); for the
        // narrow formula-entry use case, we only guard against quotes that
        // directly enclose a bracket. An unmatched quote at the start of the
        // scan flips state once and is acceptable for highlighting.
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if is_close_bracket(ch) {
            depth += 1;
            continue;
        }
        if is_open_bracket(ch) {
            depth -= 1;
            if depth == 0 {
                if matches_pair(ch, close_char) {
                    return Some(idx);
                }
                return None;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_caret_on_opening_paren() {
        let text = "=SUM(1,2)";
        let pair = bracket_pair_for_caret(text, 4).expect("matched");
        assert_eq!(pair.open_offset, 4);
        assert_eq!(pair.close_offset, 8);
    }

    #[test]
    fn matches_caret_on_closing_paren() {
        let text = "=SUM(1,2)";
        let pair = bracket_pair_for_caret(text, 8).expect("matched");
        assert_eq!(pair.open_offset, 4);
        assert_eq!(pair.close_offset, 8);
    }

    #[test]
    fn matches_caret_after_closing_paren() {
        let text = "=SUM(1,2)";
        let pair = bracket_pair_for_caret(text, 9).expect("matched");
        assert_eq!(pair.open_offset, 4);
        assert_eq!(pair.close_offset, 8);
    }

    #[test]
    fn handles_nested_mixed_bracket_kinds() {
        let text = "=FN(A[1]+(2))";
        let pair_outer = bracket_pair_for_caret(text, 3).expect("outer");
        assert_eq!(pair_outer.open_offset, 3);
        assert_eq!(pair_outer.close_offset, 12);

        let pair_inner_square = bracket_pair_for_caret(text, 5).expect("square");
        assert_eq!(pair_inner_square.open_offset, 5);
        assert_eq!(pair_inner_square.close_offset, 7);

        let pair_inner_paren = bracket_pair_for_caret(text, 9).expect("inner paren");
        assert_eq!(pair_inner_paren.open_offset, 9);
        assert_eq!(pair_inner_paren.close_offset, 11);
    }

    #[test]
    fn ignores_brackets_inside_string_literals() {
        let text = "=CONCAT(\"(hello)\",1)";
        // Caret on the string's opening paren should not match.
        assert!(bracket_pair_for_caret(text, 9).is_none());
        // Outer paren still matches.
        let outer = bracket_pair_for_caret(text, 7).expect("outer");
        assert_eq!(outer.open_offset, 7);
        assert_eq!(outer.close_offset, 19);
    }

    #[test]
    fn returns_none_when_caret_not_on_a_bracket() {
        let text = "=SUM(1,2)";
        assert!(bracket_pair_for_caret(text, 0).is_none());
        assert!(bracket_pair_for_caret(text, 2).is_none());
    }

    #[test]
    fn returns_none_when_pair_is_unbalanced() {
        let text = "=SUM(1,2";
        assert!(bracket_pair_for_caret(text, 4).is_none());
    }
}
