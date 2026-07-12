*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: signature-help past-`)` guard never fires (regression)

Status: **LANDED 2026-05-06** — OxFml's
`closed_call_close_paren_end` now walks one level deep into the
`ArgumentList` child to find `RParen`. Host-side regression tests
in `src/dnaonecalc-host/tests/scenarios/typing.rs`
(`signature_help_disappears_when_caret_is_past_close_paren` and
`signature_help_disappears_after_caret_moves_past_close_paren`)
have been un-ignored and now pass.
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / Editor regression report
Filed date: 2026-05-05
Landing date: 2026-05-06
Related:
  `docs/HANDOFF_OXFML_LET_LAMBDA_AND_SIGNATURE_HELP.md` (the prior
  past-`)` handoff whose fix never actually fired),
  `OxFml/crates/oxfml_core/src/language_service/mod.rs::closed_call_close_paren_end`,
  `OxFml/crates/oxfml_core/src/syntax/parser.rs::parse_call_expr` /
  `parse_argument_list`.

## Provenance note

Prior signature-help past-`)` work landed in
`OxFml/d069b5c` and was retroactively captured in
`docs/HANDOFF_OXFML_LET_LAMBDA_AND_SIGNATURE_HELP.md`. The fix as
shipped does not work — the structural assumption it makes about
where `RParen` lives in the green tree is wrong. The user reported
the popup is still showing past `)`, and a host-side end-to-end
test against `LiveOxfmlBridge` confirms it.

## Symptom

Type `=SUM(1,2,3)` in the DnaOneCalc home shell, leave the caret
right after the closing `)` (offset 11 in the 11-char text). The
signature-help popup `SUM(number1, [number2], ...)` keeps rendering.

End-to-end repro test (host-side, runs against a real
`LiveOxfmlBridge`):
`tests/scenarios/typing.rs::signature_help_disappears_when_caret_is_past_close_paren`.
The test currently fails with:

```
expected signature help suppressed once caret is past `)`;
document still carries: Some(SignatureHelpContext {
  callee_text: "SUM",
  call_span: TextSpan { start: 1, len: 10 },
  active_argument_index: 2,
  invocation_kind: CallExpr,
})
```

`call_span.start + call_span.len = 11`, the cursor is at 11 — the
guard *should* return `None` but the helper that drives it returns
`None` (no close-paren found) so the guard is bypassed.

## Root cause

`signature_help_context_at_cursor` runs:

```rust
if let Some(close_paren_end) = closed_call_close_paren_end(call_node) {
    if cursor_offset >= close_paren_end {
        return None;
    }
}
```

`closed_call_close_paren_end(call_node)` searches `call_node.children`
for an `RParen` token:

```rust
fn closed_call_close_paren_end(call_node: &GreenNode) -> Option<usize> {
    call_node.children.iter().rev().find_map(|child| match child {
        GreenChild::Token(token) if token.kind == TokenKind::RParen
            => Some(token.span.end()),
        _ => None,
    })
}
```

But the parser (`parse_call_expr` /
`parse_argument_list` in `crates/oxfml_core/src/syntax/parser.rs`)
builds the green tree as:

```
CallExpr
├── Token(Ident "SUM")
└── Node(ArgumentList)
    ├── Token(LParen)
    ├── …args…
    └── Token(RParen)        ← lives here, NOT under CallExpr
```

`call_node.children` for a `CallExpr` is just
`[Token(Ident), Node(ArgumentList)]`. There is no direct `RParen`
child, so `closed_call_close_paren_end` always returns `None` and the
past-`)` guard is unreachable.

## Suggested fix

Look inside the `ArgumentList` (or analogous `BraceList` for
`InvokeExpr` brace forms) when searching for the close paren:

```rust
fn closed_call_close_paren_end(call_node: &GreenNode) -> Option<usize> {
    // Walk into the call's argument-list (the only child that
    // carries the RParen token in the parser's green-tree shape).
    // Fall back to direct-child search for forms where the close
    // delimiter is a direct CallExpr / InvokeExpr child.
    fn rparen_end_in_node(node: &GreenNode) -> Option<usize> {
        node.children.iter().rev().find_map(|child| match child {
            GreenChild::Token(token) if token.kind == TokenKind::RParen => {
                Some(token.span.end())
            }
            _ => None,
        })
    }

    if let Some(end) = rparen_end_in_node(call_node) {
        return Some(end);
    }
    call_node.children.iter().rev().find_map(|child| match child {
        GreenChild::Node(node) => rparen_end_in_node(node),
        _ => None,
    })
}
```

(The walk only needs one level deep — the parser places the close
paren in the immediate argument-list child.)

## Required tests in `oxfml_core::language_service::tests`

1. `signature_help_context_returns_none_when_cursor_past_close_paren`
   — `=SUM(1,2,3)` at cursor offset 11 (one past `)`) returns `None`.
2. `signature_help_context_returns_none_at_close_paren_end`
   — `=SUM(1,2,3)` at cursor offset 10 (sitting *on* `)`) — choose
   the project's preferred boundary semantics and pin it. Today the
   helper uses `>=`, which means cursor=10 → `signature_help` is None
   when close_paren_end == 10; cursor=11 → None when close_paren_end
   == 11. The active boundary depends on whether the `RParen` span
   end is end-inclusive or end-exclusive in the project's
   `TextSpan`. Pin both sides explicitly so a future refactor can't
   silently shift the boundary.
3. `signature_help_context_still_shows_inside_open_call`
   — `=SUM(1,2,3)` at cursor offset 9 (between `3` and `)`) returns
   `Some` with `active_argument_index = 2`.
4. `signature_help_context_still_shows_in_unclosed_call`
   — `=SUM(1,2,3` (no close paren) at cursor offset 10 returns `Some`
   so the user keeps getting help while authoring.

## Direct application in working tree

Per the prior precedent in
`docs/HANDOFF_OXFML_LET_LAMBDA_AND_SIGNATURE_HELP.md`, the patch is
applied to the `OxFml` working tree alongside this handoff so the
DnaOneCalc end-to-end repro turns green immediately. The OxFml
follow-up is to land the test corpus above and confirm the helper
shape.

## DnaOneCalc-side impact

Once the OxFml fix lands, the host's existing
`project_signature_help` Just Works — the document carries
`signature_help: None` past `)` and the projection short-circuits
correctly. No host SEAM is needed.
