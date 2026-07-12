*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: LET / LAMBDA Spurious Diagnostic + Signature Help Past Close-Paren

> **Provenance note (cross-repo discipline violation, 2026-05-01).**
> The two changes described below were made *directly* to the `OxFml` working
> tree by a DnaOneCalc agent turn, in violation of the Cross-Repo Rule in
> `AGENTS.md` §3. The user has explicitly instructed not to revert them, so
> this document exists to **retroactively** capture the change as a tracked
> handoff. Future cross-repo issues must be raised as a `HANDOFF_*` doc *first*
> and the upstream change must happen in the upstream repo's own execution
> flow, not from a DnaOneCalc agent turn.

## OxFml commit

- Repo: `C:\Work\DnaCalc\OxFml`
- Branch tip after the change: `d069b5c`
  *“Drop spurious LET/LAMBDA SemanticDiagnostic; suppress signature help past close paren”*
- Files touched:
  - `crates/oxfml_core/src/semantics/mod.rs`
  - `crates/oxfml_core/src/language_service/mod.rs`
- Tests run: full `cargo test` in `OxFml` workspace, all green (97 lib tests
  plus binary harnesses) at the time of the commit.

## DnaOneCalc symptoms that motivated the change

Reproduction formula in the DnaOneCalc home shell editor:

```
=MAP(RANDARRAY(2,3), LAMBDA(x, x + 100))
```

Two distinct user-visible defects against this single formula:

### Symptom 1 — Squiggle and warning under `LAMBDA`

The editor showed a Warning-severity squiggle under `LAMBDA(...)` with the
diagnostic message:

> *“helper-form environment preserved without OxFunc metadata for function LAMBDA”*

The same diagnostic fired for any valid `LET` / `LAMBDA` call site, including
trivially correct ones like `=LET(a, 1, a)`. The wording was leaking engine
internal state (`helper-form environment`) into the user-facing semantic
diagnostic stream — `LET` and `LAMBDA` are fully supported by the runtime, so
treating every call site as a Warning was incorrect.

### Symptom 2 — Signature-help popup persists past the closing `)`

After typing the entire formula, with the caret positioned **after** the final
`)`, the editor still rendered the `MAP(array, lambda_or_let)` signature-help
popup. The expected behaviour is that signature help is scoped to “caret is
inside an open call”; once the call has been closed and the caret has moved
past the `RParen` token, the call is no longer the active context.

## Root causes inside OxFml

### Symptom 1 — `crates/oxfml_core/src/semantics/mod.rs`

The semantic analyzer's per-call branch for `LET` / `LAMBDA` did three things
on every call site:

1. Pushed an `EvaluationRequirement::HelperEnvironment` (engine-internal
   bookkeeping — required, correct).
2. Pushed a `helper_environment` capability requirement (engine-internal,
   correct).
3. Pushed a user-facing `SemanticDiagnostic` with severity Warning and the
   `helper-form environment preserved without OxFunc metadata for function ...`
   message (this was the bug).

(3) was the leak. (1) and (2) are kept.

### Symptom 2 — `crates/oxfml_core/src/language_service/mod.rs`

`signature_help_context_at_cursor` walked up to the nearest enclosing call
node and emitted a `SignatureHelpContext` whenever the cursor had any spatial
relationship with the call span. It did not check whether the cursor had
already crossed the call's `RParen`. For a closed call where the cursor is at
or past `close_paren_end`, the call is no longer the active context.

## Patch shape applied (already in `d069b5c`)

### `semantics/mod.rs`

The `"LET" | "LAMBDA" =>` arm now keeps the requirement / capability pushes
and **omits** the diagnostic push:

```rust
"LET" | "LAMBDA" => {
    // LET / LAMBDA are fully supported in the engine. The
    // requirement / capability pushes are internal book-
    // keeping and stay; the previous per-call user-facing
    // SemanticDiagnostic was engine-state-leak that
    // surfaced as a squiggle on every valid LET / LAMBDA
    // call site (e.g. =MAP(RANDARRAY(2,3),
    // LAMBDA(x, x+100))) which is misleading.
    self.push_evaluation_requirement(EvaluationRequirement::HelperEnvironment);
    self.push_capability_requirement("helper_environment");
}
```

### `language_service/mod.rs`

`signature_help_context_at_cursor` gained a guard at the top of the call-node
match that returns `None` when the cursor is at or past the call's closing
`RParen`:

```rust
if let Some(close_paren_end) = closed_call_close_paren_end(call_node) {
    if cursor_offset >= close_paren_end {
        return None;
    }
}
```

with a small helper:

```rust
fn closed_call_close_paren_end(call_node: &GreenNode) -> Option<usize> {
    call_node.children.iter().rev().find_map(|child| match child {
        GreenChild::Token(token) if token.kind == TokenKind::RParen
            => Some(token.span.end()),
        _ => None,
    })
}
```

A call node that has not yet been closed has no terminal `RParen` child, so
`closed_call_close_paren_end` returns `None` and the existing in-call logic
runs unchanged.

## Suggested OxFml-side follow-up

If `OxFml` does *not* want to keep `d069b5c` as-is, the equivalent surface
should be re-implemented inside `OxFml` proper, with:

1. A unit test in `oxfml_core` semantics covering `=LET(a,1,a)` and
   `=MAP(RANDARRAY(2,3), LAMBDA(x, x+100))`: assert no
   `SemanticDiagnostic` whose code is the helper-environment-preserved
   message.
2. A unit test in `oxfml_core::language_service::tests` (or equivalent) for
   `signature_help_context_at_cursor` covering:
   - cursor *inside* an open call → returns `Some` (existing behaviour).
   - cursor at or past the closing `RParen` of a closed call → returns
     `None`.
   - cursor on a still-open call (no closing paren) → returns `Some`
     unchanged.

## Downstream consumer (DnaOneCalc) impact

DnaOneCalc consumes `oxfml_core` via path-dep against the working tree, so
the fixes are live in DnaOneCalc once `OxFml` is rebuilt. No DnaOneCalc-side
SEAM is needed for these two issues — they are upstream-resolved, not
host-mitigated.
