*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Formula Drill Trace Model For OneCalc X-Ray

Status: superseded by completed OxFml W076 handoff
`docs/handoffs/HANDOFF-DNAONECALC-013_W076_FORMULA_DRILL_TRACE_RUNTIME_PROJECTION.md`.
DnaOneCalc has consumed the completed surface; this file is retained as the
original host-side symptom report and intake rationale.

## Summary

DnaOneCalc's formula drill-down regressed from a useful "explain how this formula evaluated" surface into a mostly raw prepared-call display. The current OxFml runtime trace is valuable, but it is not yet the right host-facing artifact for an end-user formula drill-down.

The current `RuntimeFormulaResult.evaluation.trace.prepared_calls` surface gives DnaOneCalc:

1. a flat list of `PreparedCall`,
2. each call's `function_name`,
3. each call's `returned_value`,
4. each `PreparedArgument` ordinal,
5. each argument's `resolved_value` when available,
6. coarse preparation/evaluation metadata.

That is enough to show some computed values for simple formulas. It is not enough to show a nested expression tree, branch choice, source-code focus, argument names, LET binding flow, skipped expressions, coercion steps, or error causality. DnaOneCalc should not reconstruct those from text or from private OxFml internals. OxFml owns the formula semantics and should provide a drill-ready evaluation artifact.

## Host Symptom

DnaOneCalc now has a repo-local audit scaffold:

```powershell
cargo run -p dnaonecalc-host -- audit-formula-drill --format markdown
```

It drives the real `LiveOxfmlBridge`, opens the formula drill, projects the home-shell view-model, and renders the text DnaOneCalc currently shows in User and Developer modes.

Representative current output:

```text
formula: =SUM(IF(TRUE,2,3),4)

User display:
  IF = 2
    arg[0] = TRUE
    arg[1] = 2
    arg[2] = eval=EagerValue
  SUM = 6
    arg[0] = 2
    arg[1] = 4
```

Problems visible to an end user:

1. `IF` and `SUM` are top-level siblings even though `IF(TRUE,2,3)` is the first argument of `SUM`.
2. The third `IF` branch leaks `eval=EagerValue` instead of "not used", "skipped", or another semantic state.
3. Arguments are labelled `arg[0]`, `arg[1]`, not `logical_test`, `value_if_true`, `number1`, etc.
4. The row order is evaluation order, not expression structure, but the UI presents it as a tree.
5. There are no source spans, so DnaOneCalc cannot highlight the corresponding formula text on hover/click.
6. There is no error causality. For `=1/0`, DnaOneCalc can show `OP_DIVIDE = #DIV/0!`, but not "right operand was zero" or a source-focused failure reason.

Another representative output:

```text
formula: =LET(x,1,y,2,SUM(x,y))

User display:
  SUM = 3
    arg[0] = 1
    arg[1] = 2
  LET = 3
    arg[0] = eval=EagerValue
    arg[1] = 1
    arg[2] = eval=EagerValue
    arg[3] = 2
    arg[4] = 3
```

Problems:

1. `SUM(x,y)` appears before the `LET` frame, then `LET` appears as a sibling.
2. `x` and `y` name slots appear as debug fallback text.
3. There is no binding story: `x := 1`, `y := 2`, body `SUM(x,y) := 3`.
4. The view does not help a user debug a wrong name, wrong binding, or unexpected LET result.

Incomplete formula output:

```text
formula: =SUM(

User display:
  diagnostics:
    error: expected ')'
    error: built-in function call 'SUM' rejects 0 arguments...
  tree:
    CellEntry = =SUM(
```

Problems:

1. Diagnostics are visible, but there is no failing node for the user to focus.
2. There is no source-span correlation from the diagnostics into the drill tree.
3. The drill tree cannot show the partial call shape and missing argument slot.

## Current OxFml Surface Diagnosis

The current relevant upstream structs are:

```rust
pub struct RuntimeFormulaResult {
    pub evaluation: EvaluationOutput,
    pub published_worksheet_value: EvalValue,
    pub trace_events: Vec<TraceEvent>,
    ...
}

pub struct EvaluationTrace {
    pub prepared_calls: Vec<PreparedCall>,
}

pub struct PreparedCall {
    pub function_name: String,
    pub function_id: &'static str,
    pub arg_preparation_profile: ArgPreparationProfile,
    pub prepared_arguments: Vec<PreparedArgument>,
    pub returned_value: Option<EvalValue>,
    ...
}

pub struct PreparedArgument {
    pub ordinal: usize,
    pub structure_class: PreparedStructureClass,
    pub source_class: PreparedSourceClass,
    pub evaluation_mode: PreparedEvaluationMode,
    pub reference_target: Option<String>,
    pub opaque_reason: Option<String>,
    pub resolved_value: Option<EvalValue>,
    ...
}
```

This is a prepared-call trace, not an end-user expression trace. It lacks several things DnaOneCalc needs and should not fabricate:

1. expression tree parent/child relationships,
2. source spans for calls, arguments, operators, literals, and diagnostics,
3. display labels suitable for end users,
4. argument names and semantic argument roles,
5. branch role and branch disposition (`taken`, `skipped`, `not evaluated`, `error`),
6. LET/LAMBDA binding records,
7. operator-specific labels and error explanations,
8. coercion and preparation before/after values,
9. causal links from final error to failing expression,
10. stable node ids suitable for replay and UI focus.

## Requirement

OxFml should expose a drill-ready formula evaluation artifact, either as a new field on `RuntimeFormulaResult` or as a stable projection service over the existing semantic/evaluation internals.

Suggested field:

```rust
pub struct RuntimeFormulaResult {
    pub formula_drill_trace: Option<FormulaDrillTrace>,
    ...
}
```

Suggested artifact:

```rust
pub struct FormulaDrillTrace {
    pub schema_id: &'static str,                 // "oxfml.formula_drill_trace.v1"
    pub formula_stable_id: String,
    pub source_text: String,
    pub root_node_id: FormulaDrillNodeId,
    pub nodes: Vec<FormulaDrillTraceNode>,
    pub evaluation_order: Vec<FormulaDrillNodeId>,
    pub diagnostics: Vec<FormulaDrillDiagnosticLink>,
    pub final_value: EvalValue,
}

pub struct FormulaDrillTraceNode {
    pub node_id: FormulaDrillNodeId,
    pub parent_node_id: Option<FormulaDrillNodeId>,
    pub source_span: Option<TextSpan>,
    pub expression_text: Option<String>,
    pub kind: FormulaDrillNodeKind,
    pub function_id: Option<String>,
    pub function_surface_name: Option<String>,
    pub operator_kind: Option<String>,
    pub argument_ordinal: Option<usize>,
    pub argument_name: Option<String>,
    pub argument_role: Option<FormulaArgumentRole>,
    pub label_user: String,
    pub label_developer: String,
    pub evaluation_state: FormulaDrillEvaluationState,
    pub value_before_coercion: Option<EvalValue>,
    pub value_after_coercion: Option<EvalValue>,
    pub returned_value: Option<EvalValue>,
    pub published_value: Option<EvalValue>,
    pub error: Option<FormulaDrillError>,
    pub child_node_ids: Vec<FormulaDrillNodeId>,
    pub prepared_call_index: Option<usize>,
    pub prepared_argument_index: Option<usize>,
}

pub enum FormulaDrillNodeKind {
    FormulaRoot,
    FunctionCall,
    OperatorCall,
    Argument,
    Literal,
    NameReference,
    LetBinding,
    LambdaBinding,
    ArrayLiteral,
    SpillRange,
    Error,
    DiagnosticPlaceholder,
}

pub enum FormulaDrillEvaluationState {
    Evaluated,
    Bound,
    Skipped,
    ShortCircuited,
    Omitted,
    Pending,
    Blocked,
    Error,
}

pub enum FormulaArgumentRole {
    LogicalTest,
    ValueIfTrue,
    ValueIfFalse,
    Number,
    Text,
    NameSlot,
    ValueSlot,
    BodyExpression,
    Array,
    Rows,
    Columns,
    Step,
    Other(String),
}

pub struct FormulaDrillError {
    pub code: Option<String>,          // e.g. "#DIV/0!"
    pub message: String,              // e.g. "division by zero"
    pub causal_node_id: Option<FormulaDrillNodeId>,
}

pub struct FormulaDrillDiagnosticLink {
    pub diagnostic_id: String,
    pub node_id: Option<FormulaDrillNodeId>,
    pub source_span: TextSpan,
    pub message: String,
}
```

The exact Rust names can differ. The key requirement is that OxFml exposes a stable, semantic, drill-ready tree, not just a flat call trace.

## Required Semantics

### 1. Structure View

The artifact must preserve expression structure:

```text
=SUM(IF(TRUE,2,3),4)

Formula = 6
  SUM = 6
    number1: IF(TRUE,2,3) = 2
      IF = 2
        logical_test: TRUE = TRUE
        value_if_true: 2 = 2
        value_if_false: 3 skipped
    number2: 4 = 4
```

This is different from evaluation order. Evaluation order can be a secondary list:

```text
1. IF(TRUE,2,3) -> 2
2. SUM(2,4) -> 6
```

Both are useful; the tree must not flatten nested calls into siblings.

### 2. Branch Disposition

For IF/IFS/CHOOSE/SWITCH-like behavior, every branch slot should be represented with a semantic disposition:

1. `taken`,
2. `skipped`,
3. `not reached`,
4. `error while choosing`,
5. `error while evaluating branch`.

For:

```text
=IF(FALSE,SUM(1,2),SUM(3,4))
```

Expected structure:

```text
IF = 7
  logical_test: FALSE = FALSE
  value_if_true: SUM(1,2) skipped
  value_if_false: SUM(3,4) = 7
    SUM = 7
      number1: 3 = 3
      number2: 4 = 4
```

The skipped branch must not leak `eval=EagerValue`.

### 3. LET Binding Flow

For:

```text
=LET(x,1,y,2,SUM(x,y))
```

Expected structure:

```text
LET = 3
  bind x := 1
  bind y := 2
  body: SUM(x,y) = 3
    SUM = 3
      number1: x = 1
      number2: y = 2
```

Name slots should be represented as name/binding nodes, not eager-value debug rows.

### 4. Error Causality

For:

```text
=1/0
```

Expected structure:

```text
Formula = #DIV/0!
  divide = #DIV/0!
    left: 1 = 1
    right: 0 = 0
    error: division by zero
```

The final error should link to the smallest causal node.

### 5. Source Spans And Stable Node IDs

Every drill node that corresponds to formula text should carry a source span:

```rust
pub struct TextSpan {
    pub start: usize,
    pub len: usize,
}
```

Spans let DnaOneCalc:

1. highlight formula text when the user hovers a drill row,
2. scroll to the source subexpression when the user clicks a row,
3. focus the failing expression for diagnostics,
4. retain replay-stable UI focus in evidence.

IDs should be stable within a parse/evaluation result and deterministic for the same formula/runtime context where possible.

### 6. Argument Names

Argument rows should expose names from OxFunc metadata where available:

```text
SUM
  number1
  number2

IF
  logical_test
  value_if_true
  value_if_false
```

Fallback to ordinal is acceptable only when no metadata exists, and the artifact should flag that fallback explicitly.

### 7. Coercion And Preparation Detail

Developer view needs to explain how values were prepared:

```text
arg number1
  source_class: literal
  preparation: EagerValue
  before_coercion: "1"
  after_coercion: 1
  blankness: NonBlank
```

User view should usually show only the after/prepared value, with an affordance to expand the preparation detail.

### 8. Diagnostics Linked To Nodes

For incomplete or invalid formulas, the drill trace should still expose a partial tree where possible:

```text
SUM pending
  number1: missing
diagnostic: expected ')'
diagnostic: SUM needs at least 1 argument
```

Diagnostics should link to either a node id or source span.

### 9. Rich Values And Arrays

For arrays and rich values, the drill node should expose:

1. shape,
2. compact preview,
3. whether preview is truncated,
4. route to a typed value drill/cell preview,
5. returned-value capability facts that apply to that node.

The host should not parse display strings to recover shape.

## Minimal Acceptance Corpus

OxFml should provide native tests or fixture outputs for at least:

1. `=SUM(1,2,3)`
   - one `SUM` node with argument names and returned value `6`.
2. `=SUM(IF(TRUE,2,3),4)`
   - `SUM` is root call, `IF` is nested under `SUM` argument 1, skipped false branch is explicit.
3. `=IF(FALSE,SUM(1,2),SUM(3,4))`
   - false branch is evaluated, true branch is skipped, tree and evaluation order both available.
4. `=LET(x,1,y,2,SUM(x,y))`
   - binding nodes for `x` and `y`, body node, references resolve visibly.
5. `=1/0`
   - final `#DIV/0!` links to divide node and right operand zero.
6. `=SEQUENCE(2,2)`
   - array shape and preview are typed, not display-string-only.
7. `=SUM(`
   - partial call and diagnostics linked to node/span.

## What DnaOneCalc Should Not Do

DnaOneCalc should not:

1. reconstruct a parse tree from formula text,
2. infer IF branch semantics from argument positions,
3. invent LET binding nodes from `arg[n]` rows,
4. parse `eval=...` debug strings,
5. assign argument names locally from a private registry mirror,
6. guess error causality from final error values.

Those are formula/evaluation semantics and belong in OxFml/OxFunc-owned surfaces.

## Host Coordination After OxFml Lands This

Once OxFml exposes the drill trace:

1. update DnaOneCalc's `LiveOxfmlBridge` to map `formula_drill_trace` instead of flattening `prepared_calls`,
2. update `audit-formula-drill` snapshots so nested calls are represented structurally,
3. remove local warnings that classify nested calls as top-level siblings,
4. update browser tests to assert row source spans and cross-highlight,
5. update the OneCalc UX spec status from "requires OxFml trace model" to "implemented against OxFml trace model",
6. close this handoff.

## Reproduction Path

From `C:\Work\DnaCalc\DnaOneCalc`:

```powershell
cargo run -p dnaonecalc-host -- audit-formula-drill --format markdown
```

The command is intentionally downstream-only and does not write to OxFml.
