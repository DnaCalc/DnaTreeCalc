# Formula Drill-Down Improvement Spec

Status: intake updated after OxFml W076 drill-trace completion  
Owner repo: DnaOneCalc  
Upstream dependency: fulfilled by OxFml `docs/handoffs/HANDOFF-DNAONECALC-013_W076_FORMULA_DRILL_TRACE_RUNTIME_PROJECTION.md`

## 1. Problem Statement

The formula drill-down should be the main "Live Formula Semantic X-Ray" surface for an Excel user who asks:

1. what did this formula evaluate to,
2. which inner expression produced the surprising value,
3. which branch ran,
4. where did an error begin,
5. how did a name or argument resolve,
6. what changed when I edit the formula.

The original regression came from rendering a tree-shaped UI over a mostly flat prepared-call trace. OxFml W076 now supplies `RuntimeFormulaResult.formula_drill_trace` and `RuntimeEnvironment::formula_drill_trace_for_source(...)`; DnaOneCalc must treat that trace as the source of truth and keep the remaining UX work focused on presentation, focus, and interaction.

The audit scaffold added for this work is:

```powershell
cargo run -p dnaonecalc-host -- audit-formula-drill --format markdown
```

It renders the current User and Developer drill text for a representative corpus using the real bridge and the real home-shell view-model. After the W076 intake, the audit verifies structural nesting, skipped branches, partial traces for incomplete formulas, and user/developer separation.

## 2. Current Findings

### 2.1 Simple formulas are partly useful

Current output:

```text
=SUM(1,2,3)

tree:
  SUM = 6
    arg[0] = 1
    arg[1] = 2
    arg[2] = 3
status: evaluated in 1 prepared call(s)
capability context:
  SUM | semantic profile active | FUNC.SUM
```

Useful:

1. top-level function result is visible,
2. argument values are visible,
3. final result aligns with the result hero.

Not useful enough:

1. argument labels are `arg[n]`, not `number1`, `number2`, etc.,
2. capability metadata is mixed into the primary drill flow,
3. the status line says "prepared calls", which is engine phrasing rather than user phrasing.

### 2.2 Nested formulas are misleading

Current output:

```text
=SUM(IF(TRUE,2,3),4)

tree:
  IF = 2
    arg[0] = TRUE
    arg[1] = 2
    arg[2] = eval=EagerValue
  SUM = 6
    arg[0] = 2
    arg[1] = 4
```

This is not a useful explanation. The user expects `SUM` to be the outer call and `IF` to appear inside its first argument. The current rows say "tree" but show evaluation-order siblings.

### 2.3 Branching and LET are not debuggable

Current output for a skipped branch:

```text
=IF(FALSE,SUM(1,2),SUM(3,4))

tree:
  SUM = 7
    arg[0] = 3
    arg[1] = 4
  IF = 7
    arg[0] = FALSE
    arg[1] = eval=EagerValue
    arg[2] = 7
```

Current output for LET:

```text
=LET(x,1,y,2,SUM(x,y))

tree:
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

The user cannot see:

1. which branch ran,
2. which branch was skipped,
3. how LET names were bound,
4. where `x` and `y` resolved,
5. why a branch or binding produced a value.

### 2.4 Error cases lack causality

Current output:

```text
=1/0

tree:
  OP_DIVIDE = #DIV/0!
    arg[0] = 1
    arg[1] = 0
```

This is close, but not enough. The drill should point to the causal expression and say why:

```text
divide = #DIV/0!
  left: 1
  right: 0
  error: division by zero
```

### 2.5 Incomplete formulas lack a partial semantic focus

Current output:

```text
=SUM(

diagnostics:
  error: expected ')'
  error: built-in function call 'SUM' rejects 0 arguments...
tree:
  CellEntry = =SUM(
```

The diagnostics are visible, but the drill does not show a partial `SUM` node, missing argument slot, or a row that can be focused from the diagnostic.

## 3. Product Goals

### 3.1 User mode

User mode is for someone debugging an Excel formula, not an engine maintainer. It should:

1. show the formula as an explainable expression tree,
2. show partial results at each meaningful expression,
3. show branch decisions and skipped branches,
4. show name bindings and resolved names,
5. show the smallest causal expression for errors,
6. keep engine metadata behind secondary disclosure.

### 3.2 Developer mode

Developer mode is for engine and seam investigation. It should:

1. expose OxFml node ids, source spans, function ids, and preparation metadata,
2. show evaluation order separately from expression structure,
3. show raw trace references without polluting User mode,
4. make upstream gaps visible with explicit seam ids.

### 3.3 Non-goals

The drill-down must not:

1. become a worksheet dependency browser,
2. invent formula semantics inside DnaOneCalc,
3. parse OxFml debug strings into meaning,
4. hide unsupported upstream gaps,
5. make capability metadata compete with the formula explanation.

## 4. Target UX

### 4.1 First viewport

The drill opens between editor and result, but the first visible panel should be compact and explanation-first:

```text
+------------------------------------------------------------------+
| Formula drill                                  [User] [Dev] [x]   |
| =SUM(IF(TRUE,2,3),4)                                      = 6     |
|                                                                  |
|  [Structure] [Steps] [Problems]                                  |
|                                                                  |
|  SUM                                             6                |
|  |-- number1: IF(TRUE,2,3)                       2                |
|  |   |-- logical_test: TRUE                      TRUE             |
|  |   |-- value_if_true: 2                        2       taken    |
|  |   '-- value_if_false: 3                       skipped          |
|  '-- number2: 4                                  4                |
|                                                                  |
|  evaluated 2 calls | 0 problems | deterministic context           |
+------------------------------------------------------------------+
```

Key decisions:

1. the outer expression is visually first,
2. child expressions are nested under the argument where they appear,
3. argument names replace `arg[n]`,
4. branch disposition is visible,
5. final value is repeated in the header for orientation,
6. capability data is not in the main flow.

### 4.2 Structure tab

Default view. Shows expression structure and values.

```text
Formula = 6
  SUM = 6
    number1: IF(TRUE,2,3) = 2
      IF = 2
        logical_test: TRUE = TRUE
        value_if_true: 2 = 2              [taken]
        value_if_false: 3                 [skipped]
    number2: 4 = 4
```

Row layout:

```text
[expand] [label/expression]                  [value]        [state]
```

Examples:

```text
v SUM                                          6             ok
  v number1: IF(TRUE,2,3)                     2             ok
    - logical_test: TRUE                      TRUE          ok
    - value_if_true: 2                        2             taken
    - value_if_false: 3                       skipped       skipped
  - number2: 4                                4             ok
```

### 4.3 Steps tab

Shows evaluation order when the user needs to understand sequencing:

```text
+----------------------+-------------------------------------------+
| Step                 | Result                                    |
+----------------------+-------------------------------------------+
| 1 IF(TRUE,2,3)       | 2 (true branch)                           |
| 2 SUM(2,4)           | 6                                         |
+----------------------+-------------------------------------------+
```

The Structure tab answers "where is this in my formula?" The Steps tab answers "what ran first?"

### 4.4 Problems tab

Only appears or badges when diagnostics/errors exist:

```text
+------------------------------------------------------------------+
| Problems (2)                                                      |
|                                                                  |
| 1. expected ')'                                                   |
|    source: after '=SUM('                                          |
|    action: complete the argument list                             |
|                                                                  |
| 2. SUM has no arguments                                           |
|    source: SUM(...)                                               |
|    action: add at least one number/value argument                  |
+------------------------------------------------------------------+
```

For runtime errors:

```text
+------------------------------------------------------------------+
| Problems (1)                                                      |
|                                                                  |
| #DIV/0! at divide                                                 |
|   =1/0                                                           |
|    ^                                                             |
| right operand is zero                                             |
+------------------------------------------------------------------+
```

### 4.5 Row focus interaction

Hover or keyboard focus on a row highlights the corresponding formula span:

```text
Editor:
  =SUM(IF(TRUE,2,3),4)
       ^^^^^^^^^^^^

Drill:
  > number1: IF(TRUE,2,3) = 2
```

Clicking a row pins the focus and opens a side detail area:

```text
+-----------------------------------+------------------------------+
| Structure                         | Detail                       |
|                                   |                              |
| > IF(TRUE,2,3) = 2                | IF                           |
|   logical_test: TRUE = TRUE       | result: 2                    |
|   value_if_true: 2 = 2            | branch: value_if_true        |
|   value_if_false: 3 skipped       | source: chars 5..17          |
+-----------------------------------+------------------------------+
```

### 4.6 Developer details

Developer mode adds trace and seam fields without changing the default reading path:

```text
+------------------------------------------------------------------+
| Developer detail: IF(TRUE,2,3)                                    |
| node_id: drill:call:1                                             |
| source_span: 5..17                                                |
| function_id: FUNC.IF                                              |
| prepared_call_index: 0                                            |
| arg_profile: refs_visible_in_adapter                              |
| semantic_kernel: semantic_kernel_metadata.v1...                   |
| admission: arg_admission_metadata.v1...                           |
+------------------------------------------------------------------+
```

Raw capability metadata moves here or to an adjacent "Context" disclosure, not the primary tree.

### 4.7 Error-first state

When a formula evaluates to an error, the drill should open with the causal node emphasized:

```text
+------------------------------------------------------------------+
| Formula drill                                      #DIV/0!         |
|                                                                  |
|  divide                                           #DIV/0!   error |
|  |-- left: 1                                      1               |
|  '-- right: 0                                     0        cause  |
|                                                                  |
| Problem: division by zero at right operand                         |
+------------------------------------------------------------------+
```

### 4.8 Incomplete formula state

For incomplete formulas, show a partial call instead of a generic cell-entry row:

```text
+------------------------------------------------------------------+
| Formula drill                                      incomplete      |
|                                                                  |
|  SUM                                             pending           |
|  '-- number1                                    missing            |
|                                                                  |
| Problem: expected ')' after argument list                          |
| Problem: SUM needs at least one argument                            |
+------------------------------------------------------------------+
```

## 5. Required Data Contract

The current DnaOneCalc `FormulaDrillNode` is too small:

```rust
pub struct FormulaDrillNode {
    pub node_id: String,
    pub label: String,
    pub value_preview: Option<String>,
    pub state: FormulaWalkNodeState,
    pub children: Vec<FormulaDrillNode>,
}
```

Target DnaOneCalc projection after OxFml trace support:

```rust
pub struct FormulaDrillNodeView {
    pub node_id: String,
    pub parent_node_id: Option<String>,
    pub source_span: Option<TextSpanView>,
    pub expression_text: Option<String>,
    pub label_user: String,
    pub label_developer: String,
    pub value_preview: Option<String>,
    pub value_kind: FormulaDrillValueKind,
    pub state: FormulaDrillNodeStateView,
    pub role: Option<String>,
    pub branch_disposition: Option<BranchDispositionView>,
    pub error_summary: Option<String>,
    pub children: Vec<FormulaDrillNodeView>,
    pub developer_detail: FormulaDrillDeveloperDetail,
}
```

DnaOneCalc may own the view shape, but the semantic fields must come from OxFml/OxFunc.

## 6. Host-Side Implementation Plan

### Phase 1: Keep audit scaffold live

1. Keep `audit-formula-drill` as a regression tool.
2. Add formulas to the corpus whenever a drill regression is reported.
3. Prefer audit output in bug reports over screenshots when the problem is row content.

Acceptance:

```powershell
cargo run -p dnaonecalc-host -- audit-formula-drill --format markdown
```

prints User and Developer displays for the default corpus.

### Phase 2: Stop mixing capability context into primary drill

Before OxFml trace improvements land, DnaOneCalc can improve one host-owned issue:

1. move capability context under a collapsed `Context` disclosure,
2. hide it entirely in the default row flow,
3. keep it available in Developer mode.

This is host UX, not upstream semantics.

### Phase 3: Improve labels where upstream already gives safe data

Allowed host improvements:

1. use friendlier operator labels where `function_name` is already `OP_DIVIDE`, `OP_ADD`, etc.,
2. render status text as "evaluated 1 step" instead of "prepared call(s)",
3. mark debug fallback rows visibly as "engine detail unavailable" instead of presenting them as values.

Not allowed:

1. invent branch semantics,
2. invent LET bindings,
3. infer expression hierarchy from formula text.

### Phase 4: Consume OxFml drill trace

Status: implemented for the current bridge/view-model path.

1. mapped OxFml trace nodes into the host formula-walk projection,
2. retained source spans and expression text on each row for focus wiring,
3. replaced ordinal-only args with OxFml semantic argument names where available,
4. moved capability context out of User mode,
5. added audit coverage for nested structure and display blocks.

Remaining UX work:

1. add source-span hover and click focus,
2. split Structure / Steps / Problems tabs,
3. add browser tests for nested structure, branch skipping, LET binding, and error causality,
4. add selected-node detail pane using the trace node ids and spans already carried through the model.

## 7. Browser And Test Requirements

Add browser invariants:

1. `drill_structure_nests_if_inside_sum_argument`
2. `drill_structure_marks_skipped_if_branch`
3. `drill_structure_shows_let_bindings`
4. `drill_error_focuses_divide_by_zero_cause`
5. `drill_hover_highlights_formula_span`
6. `drill_click_pins_detail_panel`
7. `drill_user_mode_hides_engine_metadata`
8. `drill_developer_mode_shows_trace_ids`
9. `drill_problems_tab_links_diagnostics_to_nodes`
10. `audit_formula_drill_corpus_has_no_debug_fallback_rows_for_supported_cases`

Native scaffold tests should keep checking:

1. audit corpus renders User and Developer blocks,
2. nested formulas no longer appear as top-level prepared-call siblings,
3. incomplete formulas expose a partial formula trace rather than a single `CellEntry` fallback.

## 8. Self-Review And Rework

### 8.1 First design risk

The first tempting design is "make the current rows prettier." That is not enough. Pretty `arg[0]` rows still fail the user goal because they do not explain structure, branch choice, source focus, or causality.

Revision: the target design is explicitly split into Structure, Steps, and Problems. Structure is not derived from the flat prepared-call order.

### 8.2 Second design risk

Another tempting design is to put all OxFml metadata in the same panel for honesty. That makes the primary user path noisy and was one source of regression.

Revision: User mode shows explanation. Developer mode and a collapsed Context disclosure show metadata. Honesty remains available without crowding the first read.

### 8.3 Third design risk

The design could overreach by making DnaOneCalc reconstruct formula semantics. That would violate repo scope and become a local second evaluator.

Revision: DnaOneCalc only owns view composition and interaction. OxFml must supply the semantic trace tree. Until then, DnaOneCalc may improve labels and layout but must keep gaps visible.

### 8.4 Fourth design risk

The tree alone may not answer "what happened first?" for lazy or nested formulas.

Revision: add a Steps tab. The user can switch between formula structure and evaluation order without confusing the two.

### 8.5 Fifth design risk

Error debugging can get buried if the drill always opens at the root.

Revision: when there are diagnostics or runtime errors, the drill should emphasize the smallest causal node and badge the Problems tab.

## 9. Revised Target After Review

The improved drill-down should feel like this:

```text
User opens Ctrl+D
  |
  v
+------------------------------------------------------------------+
| Formula drill                                      result: 6       |
| =SUM(IF(TRUE,2,3),4)                                             |
|                                                                  |
| [Structure] [Steps] [Problems 0]                    [Context >]   |
|                                                                  |
| v SUM                                               6             |
|   v number1: IF(TRUE,2,3)                           2             |
|     - logical_test: TRUE                            TRUE          |
|     - value_if_true: 2                              2    taken    |
|     - value_if_false: 3                             skipped       |
|   - number2: 4                                      4             |
+------------------------------------------------------------------+

User clicks IF row
  |
  v
+-----------------------------------+------------------------------+
| v SUM                             | IF(TRUE,2,3)                 |
| > number1: IF(TRUE,2,3) = 2       | returned: 2                  |
|   logical_test: TRUE              | branch: value_if_true        |
|   value_if_true: 2                | source highlighted in editor |
|   value_if_false: skipped         |                              |
+-----------------------------------+------------------------------+

User switches to Steps
  |
  v
+------------------------------------------------------------------+
| Step 1  IF(TRUE,2,3)                         2                   |
| Step 2  SUM(2,4)                             6                   |
+------------------------------------------------------------------+
```

This revised target satisfies the intended usage:

1. simple formulas remain quick to read,
2. nested formulas show where every partial result came from,
3. lazy branches explain what did not run,
4. errors point to a cause,
5. source highlighting links the drill to the editor,
6. Developer mode still exposes raw trace/provenance detail,
7. DnaOneCalc stays within its scope by consuming OxFml semantics rather than recreating them.
