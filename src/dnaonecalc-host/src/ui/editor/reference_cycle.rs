//! F4 reference-form cycling.
//!
//! Detects a cell reference at/around the caret or spanning the selection and
//! rewrites it to the next form in the Excel cycle: `A1` -> `$A$1` -> `A$1` ->
//! `$A1` -> `A1`. Only single-cell A1 references are handled here; ranges and
//! structured references are deferred.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCycleResult {
    pub text: String,
    pub reference_start: usize,
    pub reference_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceToken {
    start: usize,
    end: usize,
    column_abs: bool,
    column_letters: String,
    row_abs: bool,
    row_digits: String,
}

/// Cycle the reference that sits at or spans the supplied character offsets.
///
/// `selection_start..selection_end` is interpreted as the caret range in
/// character (not byte) offsets. If the selection is collapsed the surrounding
/// reference is detected; otherwise the first reference contained in the
/// selection is rewritten.
pub fn cycle_reference_form(
    text: &str,
    selection_start: usize,
    selection_end: usize,
) -> Option<ReferenceCycleResult> {
    let chars: Vec<char> = text.chars().collect();
    let selection_lo = selection_start.min(selection_end).min(chars.len());
    let selection_hi = selection_start.max(selection_end).min(chars.len());
    let reference = find_reference_for_range(&chars, selection_lo, selection_hi)?;
    let rewritten = next_form(&reference);
    let new_text = rebuild_text(&chars, &reference, &rewritten);
    let new_end = reference.start + rewritten.chars().count();
    Some(ReferenceCycleResult {
        text: new_text,
        reference_start: reference.start,
        reference_end: new_end,
    })
}

fn find_reference_for_range(
    chars: &[char],
    selection_lo: usize,
    selection_hi: usize,
) -> Option<ReferenceToken> {
    // Prefer a reference that overlaps with the caret / selection. We grow
    // outward from `selection_lo` backwards until the start of a plausible
    // reference token, then forward until it terminates.
    let mut start = selection_lo;
    while start > 0 && is_reference_char(chars[start - 1]) {
        start -= 1;
    }
    let mut cursor = start;
    while cursor < chars.len() {
        if let Some(token) = try_read_reference(chars, cursor) {
            if token.end > selection_lo && token.start <= selection_hi {
                return Some(token);
            }
            cursor = token.end;
            continue;
        }
        cursor += 1;
    }
    None
}

fn is_reference_char(ch: char) -> bool {
    ch == '$' || ch.is_ascii_alphabetic() || ch.is_ascii_digit()
}

fn try_read_reference(chars: &[char], start: usize) -> Option<ReferenceToken> {
    // Guard against scanning from inside a previous identifier: the caller
    // ensures we start at the beginning of a potential reference.
    if start > 0 && is_reference_continuation(chars[start - 1]) {
        return None;
    }

    let mut idx = start;

    let column_abs = if chars.get(idx).copied() == Some('$') {
        idx += 1;
        true
    } else {
        false
    };

    let letters_start = idx;
    while idx < chars.len() && chars[idx].is_ascii_alphabetic() {
        idx += 1;
    }
    if idx == letters_start {
        return None;
    }
    let column_letters: String = chars[letters_start..idx]
        .iter()
        .map(|ch| ch.to_ascii_uppercase())
        .collect();
    // Excel columns are capped at `XFD` (three letters). Longer strings of
    // letters are almost certainly a function or identifier, not a cell ref.
    if column_letters.len() > 3 {
        return None;
    }

    let row_abs = if chars.get(idx).copied() == Some('$') {
        idx += 1;
        true
    } else {
        false
    };

    let digits_start = idx;
    while idx < chars.len() && chars[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == digits_start {
        return None;
    }
    let row_digits: String = chars[digits_start..idx].iter().collect();

    // If what follows is an identifier character the match is really the
    // prefix of a larger name like `A1B` (a defined name) — skip it.
    if chars
        .get(idx)
        .copied()
        .is_some_and(is_reference_continuation)
    {
        return None;
    }

    Some(ReferenceToken {
        start,
        end: idx,
        column_abs,
        column_letters,
        row_abs,
        row_digits,
    })
}

fn is_reference_continuation(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn next_form(token: &ReferenceToken) -> String {
    let (next_col_abs, next_row_abs) = match (token.column_abs, token.row_abs) {
        (false, false) => (true, true),
        (true, true) => (false, true),
        (false, true) => (true, false),
        (true, false) => (false, false),
    };
    let mut out = String::new();
    if next_col_abs {
        out.push('$');
    }
    out.push_str(&token.column_letters);
    if next_row_abs {
        out.push('$');
    }
    out.push_str(&token.row_digits);
    out
}

fn rebuild_text(chars: &[char], reference: &ReferenceToken, replacement: &str) -> String {
    let mut out = String::with_capacity(chars.len() + replacement.len());
    out.extend(chars[..reference.start].iter());
    out.push_str(replacement);
    out.extend(chars[reference.end..].iter());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cycle(text: &str, start: usize, end: usize) -> (String, usize, usize) {
        let result = cycle_reference_form(text, start, end).expect("reference detected");
        (result.text, result.reference_start, result.reference_end)
    }

    #[test]
    fn cycles_relative_to_absolute() {
        let (text, start, end) = cycle("=A1+B2", 1, 1);
        assert_eq!(text, "=$A$1+B2");
        assert_eq!(start, 1);
        assert_eq!(end, 5);
    }

    #[test]
    fn cycles_absolute_to_mixed_absolute_row() {
        let (text, _, _) = cycle("=$A$1+B2", 1, 1);
        assert_eq!(text, "=A$1+B2");
    }

    #[test]
    fn cycles_mixed_absolute_row_to_mixed_absolute_column() {
        let (text, _, _) = cycle("=A$1+B2", 1, 1);
        assert_eq!(text, "=$A1+B2");
    }

    #[test]
    fn cycles_mixed_absolute_column_back_to_relative() {
        let (text, _, _) = cycle("=$A1+B2", 1, 1);
        assert_eq!(text, "=A1+B2");
    }

    #[test]
    fn detects_reference_from_inside_token() {
        let (text, _, _) = cycle("=A1+B2", 2, 2);
        assert_eq!(text, "=$A$1+B2");

        let (text, _, _) = cycle("=A1+B2", 4, 4);
        assert_eq!(text, "=A1+$B$2");
    }

    #[test]
    fn ignores_caret_on_non_reference_character() {
        // Caret sits on '+' between two references - no reference is "at" the
        // caret, so cycling is a no-op.
        assert!(cycle_reference_form("=A1+B2", 3, 3).is_none());
    }

    #[test]
    fn detects_reference_covering_selection() {
        let (text, _, _) = cycle("=SUM(A1:B2)", 5, 7);
        // Selection "A1:" starts in A1; we take A1 as the first token we see.
        assert_eq!(text, "=SUM($A$1:B2)");
    }

    #[test]
    fn skips_caret_inside_defined_name_prefix() {
        // "Apple1" looks reference-shaped but has too many letters; when the
        // caret is inside it, F4 is a no-op rather than reaching ahead.
        assert!(cycle_reference_form("=Apple1+B2", 1, 1).is_none());
    }

    #[test]
    fn returns_none_when_no_reference_on_line() {
        assert!(cycle_reference_form("=SUM(1,2)", 1, 1).is_none());
        assert!(cycle_reference_form("=SUM(1,2)", 7, 7).is_none());
    }

    #[test]
    fn respects_excel_column_limit() {
        // Four letters cannot form a valid column (Excel cap is XFD).
        assert!(cycle_reference_form("=ABCDE1", 1, 1).is_none());
    }
}
