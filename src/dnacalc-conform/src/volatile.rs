//! Nondeterministic-formula detection for the `Excluded { nondeterministic }`
//! rung of the verdict ladder.
//!
//! A formula whose Excel-cached value depends on the moment or on a random
//! stream cannot be scored against the cache: the cache is Excel's answer at
//! save time, not a property of the workbook. The list is deliberately the
//! bead's list — the functions whose result is not a function of the
//! workbook's contents — not Excel's wider "volatile" set (`OFFSET`,
//! `INDIRECT`, `CELL`, `INFO` recalc on every tick but are deterministic
//! given the workbook). This scanner is one of the rung's two triggers;
//! the other is the file's own `ca="1"` flag (read in `oracle.rs`), which
//! Excel sets on every cell of a volatile closure — so an `INDIRECT` user or
//! a dependent of `TODAY()` is excluded by the flag even though this list
//! would not name it. The verdict key says which trigger fired.

/// The functions whose result is not determined by the workbook.
pub const NONDETERMINISTIC_FUNCTIONS: &[&str] =
    &["NOW", "TODAY", "RAND", "RANDBETWEEN", "RANDARRAY"];

/// The first nondeterministic function called in `text` (a formula as the
/// file stores it, with or without the leading `=`), or `None`.
///
/// A call is an identifier immediately followed (after optional spaces) by
/// `(`. Double-quoted string literals and single-quoted sheet names are
/// skipped, so `="NOW()"` and `='RAND()'!A1` are not calls. The `_xlfn.` /
/// `_xlws.` future-function prefixes Excel stores for newer functions are
/// stripped before the name is compared, so `_xlfn.RANDARRAY(` is a call to
/// `RANDARRAY`. Matching is case-insensitive.
#[must_use]
pub fn nondeterministic_call(text: &str) -> Option<&'static str> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            i = skip_quoted(&chars, i, '"');
            continue;
        }
        if c == '\'' {
            i = skip_quoted(&chars, i, '\'');
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() && chars[j] == '(' {
                let upper = name.to_ascii_uppercase();
                let bare = upper
                    .strip_prefix("_XLFN.")
                    .or_else(|| upper.strip_prefix("_XLWS."))
                    .unwrap_or(&upper);
                if let Some(found) = NONDETERMINISTIC_FUNCTIONS
                    .iter()
                    .find(|candidate| **candidate == bare)
                {
                    return Some(found);
                }
            }
            continue;
        }
        i += 1;
    }
    None
}

/// Index just past the closing quote of the literal opening at `open`
/// (doubled quotes are escapes), or the end of the text if unterminated.
fn skip_quoted(chars: &[char], open: usize, quote: char) -> usize {
    let mut i = open + 1;
    while i < chars.len() {
        if chars[i] == quote {
            if i + 1 < chars.len() && chars[i + 1] == quote {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_five_named_functions_in_any_case_and_with_the_xlfn_prefix() {
        assert_eq!(nondeterministic_call("NOW()"), Some("NOW"));
        assert_eq!(nondeterministic_call("=today()"), Some("TODAY"));
        assert_eq!(nondeterministic_call("A1+RAND()*2"), Some("RAND"));
        assert_eq!(
            nondeterministic_call("RANDBETWEEN(1, 6)"),
            Some("RANDBETWEEN")
        );
        assert_eq!(
            nondeterministic_call("_xlfn.RANDARRAY(2,2)"),
            Some("RANDARRAY")
        );
        assert_eq!(nondeterministic_call("IF(A1, NOW (), 0)"), Some("NOW"));
    }

    #[test]
    fn does_not_flag_names_that_merely_contain_the_word_or_are_not_calls() {
        assert_eq!(nondeterministic_call("RANDOM_NAME+1"), None);
        assert_eq!(nondeterministic_call("NOWHERE(1)"), None);
        assert_eq!(nondeterministic_call("my.now(1)"), None);
        assert_eq!(nondeterministic_call("TODAY"), None);
        assert_eq!(nondeterministic_call("A1*3"), None);
        assert_eq!(nondeterministic_call(""), None);
    }

    #[test]
    fn skips_string_literals_and_quoted_sheet_names() {
        assert_eq!(nondeterministic_call("\"NOW()\""), None);
        assert_eq!(nondeterministic_call("\"a\"\"NOW()\"\"b\""), None);
        assert_eq!(nondeterministic_call("'RAND()'!A1"), None);
        assert_eq!(nondeterministic_call("\"x\"&NOW()"), Some("NOW"));
        assert_eq!(nondeterministic_call("\"unterminated NOW()"), None);
    }

    #[test]
    fn offset_and_indirect_are_deterministic_and_stay_scorable() {
        assert_eq!(nondeterministic_call("OFFSET(A1,0,0)"), None);
        assert_eq!(nondeterministic_call("INDIRECT(\"A1\")"), None);
    }
}
