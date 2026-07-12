use crate::adapters::oxfml::EditorSyntaxSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTokenRole {
    Operator,
    Function,
    Number,
    Delimiter,
    Identifier,
    Text,
    /// Trivia between tokens (whitespace, comments). Rendered with no
    /// special colour but emitted as its own run so the overlay's text
    /// content equals the textarea's value character-for-character —
    /// without this the caret drifts past the trivia run width as soon
    /// as the user types `= SUM` (with a space) or any other input
    /// containing inter-token whitespace.
    Trivia,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRun {
    pub text: String,
    pub span_start: usize,
    pub span_len: usize,
    pub role: SyntaxTokenRole,
}

/// Build syntax runs from an editor snapshot, preserving trivia
/// (whitespace, comments) so the rendered overlay's character count
/// equals the textarea's content character-for-character.
///
/// Trivia attachment rule: each upstream token carries both
/// `leading_trivia` and `trailing_trivia`, and the same whitespace can
/// appear on BOTH sides of adjacent tokens (the prior token's
/// trailing-trivia equals the next token's leading-trivia). To avoid
/// double-rendering, this function:
///
/// * emits `leading_trivia` of EVERY token (covers the gap before the
///   token, which for the first token is also any leading whitespace
///   at the start of input),
/// * emits `trailing_trivia` of ONLY the LAST token (covers any
///   trailing whitespace at the end of input that has no successor
///   token to claim it as leading-trivia).
///
/// The `expected_offset` cursor advances by each emitted run's text
/// length. Gaps that the trivia doesn't fill (a defensive case for
/// malformed snapshots) leave the overlay shorter than the textarea —
/// preferable to longer/duplicated text, and caught by the
/// browser-corpus invariant `syntax_overlay_text_must_match_textarea_value_exactly`.
pub fn syntax_runs_from_snapshot(snapshot: &EditorSyntaxSnapshot) -> Vec<SyntaxRun> {
    let mut runs = Vec::with_capacity(snapshot.tokens.len() * 2 + 1);
    let mut expected_offset: usize = 0;
    let last_index = snapshot.tokens.len().saturating_sub(1);
    for (token_index, token) in snapshot.tokens.iter().enumerate() {
        for trivia in &token.leading_trivia {
            push_trivia_run(&mut runs, &mut expected_offset, &trivia.text);
        }
        runs.push(SyntaxRun {
            text: token.text.clone(),
            span_start: token.span.start,
            span_len: token.span.len,
            role: classify_token_role(&token.text),
        });
        expected_offset = token.span.start.saturating_add(token.span.len);
        if token_index == last_index {
            for trivia in &token.trailing_trivia {
                push_trivia_run(&mut runs, &mut expected_offset, &trivia.text);
            }
        }
    }
    runs
}

/// Emit a trivia run at `expected_offset`, advancing the cursor by
/// the trivia's character count. Empty-text trivia entries are
/// skipped so the run list never carries zero-width segments.
fn push_trivia_run(runs: &mut Vec<SyntaxRun>, expected_offset: &mut usize, text: &str) {
    if text.is_empty() {
        return;
    }
    let len = text.chars().count();
    runs.push(SyntaxRun {
        text: text.to_string(),
        span_start: *expected_offset,
        span_len: len,
        role: SyntaxTokenRole::Trivia,
    });
    *expected_offset = (*expected_offset).saturating_add(len);
}

pub fn syntax_runs_from_text(text: &str) -> Vec<SyntaxRun> {
    let mut runs = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut chars = text.chars().enumerate().peekable();

    while let Some((idx, ch)) = chars.next() {
        if ch.is_whitespace() {
            flush_token(&mut runs, &mut current, current_start);
            continue;
        }

        if ch == '=' || matches!(ch, '(' | ')' | ',') {
            flush_token(&mut runs, &mut current, current_start);
            runs.push(SyntaxRun {
                text: ch.to_string(),
                span_start: idx,
                span_len: 1,
                role: classify_token_role(&ch.to_string()),
            });
            continue;
        }

        if current.is_empty() {
            current_start = idx;
        }
        current.push(ch);

        if chars
            .peek()
            .map(|(_, next)| next.is_whitespace() || matches!(next, '=' | '(' | ')' | ','))
            .unwrap_or(true)
        {
            flush_token(&mut runs, &mut current, current_start);
        }
    }

    runs
}

fn flush_token(runs: &mut Vec<SyntaxRun>, current: &mut String, current_start: usize) {
    if current.is_empty() {
        return;
    }

    let text = std::mem::take(current);
    runs.push(SyntaxRun {
        span_len: text.chars().count(),
        span_start: current_start,
        role: classify_token_role(&text),
        text,
    });
}

fn classify_token_role(text: &str) -> SyntaxTokenRole {
    if text == "=" {
        SyntaxTokenRole::Operator
    } else if matches!(text, "(" | ")" | ",") {
        SyntaxTokenRole::Delimiter
    } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        SyntaxTokenRole::Number
    } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        SyntaxTokenRole::Function
    } else if !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        SyntaxTokenRole::Identifier
    } else {
        SyntaxTokenRole::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_runs_follow_snapshot_tokens() {
        let snapshot = crate::test_support::make_editor_syntax_snapshot(
            "formula-1",
            "green-1",
            vec![
                crate::test_support::make_editor_token("=", 0),
                crate::test_support::make_editor_token("SUM", 1),
            ],
        );

        let runs = syntax_runs_from_snapshot(&snapshot);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].text, "SUM");
        assert_eq!(runs[1].span_start, 1);
        assert_eq!(runs[0].role, SyntaxTokenRole::Operator);
        assert_eq!(runs[1].role, SyntaxTokenRole::Function);
    }

    #[test]
    fn syntax_runs_preserve_inter_token_whitespace_trivia() {
        // Simulate "= SUM" — the upstream parser attaches inter-token
        // whitespace as `leading_trivia` of the FOLLOWING token (the
        // browser-corpus invariant `syntax_overlay_text_must_match_textarea_value_exactly`
        // pins the attachment convention). Without trivia preservation
        // the overlay would emit "=SUM" (4 chars) instead of "= SUM"
        // (5 chars), causing the caret to drift one character-width
        // past the visible "M" — the user-reported "caret offset
        // from insertion point" bug.
        use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken, FormulaTextSpan};
        use oxfml_core::consumer::editor::{EditorTrivia, EditorTriviaKind};
        use oxfml_core::source::FormulaChannelKind;
        use oxfml_core::syntax::token::TokenKind;

        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "f".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: "g".to_string(),
            tokens: vec![
                EditorToken {
                    kind: TokenKind::Equals,
                    text: "=".to_string(),
                    leading_trivia: Vec::new(),
                    trailing_trivia: Vec::new(),
                    span: FormulaTextSpan { start: 0, len: 1 },
                },
                EditorToken {
                    kind: TokenKind::Identifier,
                    text: "SUM".to_string(),
                    leading_trivia: vec![EditorTrivia {
                        kind: EditorTriviaKind::Whitespace,
                        text: " ".to_string(),
                    }],
                    trailing_trivia: Vec::new(),
                    span: FormulaTextSpan { start: 2, len: 3 },
                },
            ],
        };

        let runs = syntax_runs_from_snapshot(&snapshot);
        let total_text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(
            total_text, "= SUM",
            "concatenated run text must equal the textarea value (5 chars)",
        );
        // Three runs: `=`, ` `, `SUM`.
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].role, SyntaxTokenRole::Operator);
        assert_eq!(runs[1].role, SyntaxTokenRole::Trivia);
        assert_eq!(runs[1].text, " ");
        assert_eq!(runs[1].span_start, 1);
        assert_eq!(runs[1].span_len, 1);
        assert_eq!(runs[2].role, SyntaxTokenRole::Function);
        assert_eq!(runs[2].span_start, 2);
    }

    #[test]
    fn syntax_runs_preserve_leading_trivia_at_start_of_input() {
        // Simulate "  =" — two-space leading trivia on the `=` token.
        use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken, FormulaTextSpan};
        use oxfml_core::consumer::editor::{EditorTrivia, EditorTriviaKind};
        use oxfml_core::source::FormulaChannelKind;
        use oxfml_core::syntax::token::TokenKind;

        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "f".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: "g".to_string(),
            tokens: vec![EditorToken {
                kind: TokenKind::Equals,
                text: "=".to_string(),
                leading_trivia: vec![EditorTrivia {
                    kind: EditorTriviaKind::Whitespace,
                    text: "  ".to_string(),
                }],
                trailing_trivia: Vec::new(),
                span: FormulaTextSpan { start: 2, len: 1 },
            }],
        };
        let runs = syntax_runs_from_snapshot(&snapshot);
        let total_text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(total_text, "  =");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].role, SyntaxTokenRole::Trivia);
        assert_eq!(runs[0].text, "  ");
        assert_eq!(runs[1].role, SyntaxTokenRole::Operator);
    }

    #[test]
    fn syntax_runs_from_text_splits_formula_like_input() {
        let runs = syntax_runs_from_text("=LET(x,1,x)");
        assert_eq!(runs.len(), 9);
        assert_eq!(runs[1].text, "LET");
        assert_eq!(runs[1].role, SyntaxTokenRole::Function);
        assert_eq!(runs[3].role, SyntaxTokenRole::Identifier);
    }
}
