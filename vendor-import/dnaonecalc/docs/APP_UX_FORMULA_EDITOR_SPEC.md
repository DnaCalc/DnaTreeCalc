# DNA OneCalc Formula Editor Spec

Status: `draft_editor_authority`
Date: 2026-04-05
Scope: custom formula editor scope, compatibility floor, staged feature set, implementation design, and TDD obligations for `DnaOneCalc`

Companion notes:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
4. [APP_UX_SCREEN_SPEC_EXPLORE.md](APP_UX_SCREEN_SPEC_EXPLORE.md)
5. [APP_UX_USE_CASES.md](APP_UX_USE_CASES.md)
6. [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md)
7. [APP_UX_HOST_STATE_SLICING.md](APP_UX_HOST_STATE_SLICING.md)
8. [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md)

Official Excel behavior references used for this note:
1. [Use AutoComplete when entering formulas](https://support.microsoft.com/en-us/office/use-autocomplete-when-entering-formulas-d51ef125-60ff-438f-ba26-d9bd6b363bbe)
2. [Formula tips and tricks](https://support.microsoft.com/en-us/office/formula-tips-and-tricks-2b93588a-e7fc-4ca5-b496-35e4141c68bd)
3. [Keyboard shortcuts in Excel](https://support.microsoft.com/en-us/office/keyboard-shortcuts-in-excel-1798d9d5-842a-42b8-9c99-9b7213f0040f)
4. [Switch between relative and absolute references](https://support.microsoft.com/en-us/office/switch-between-relative-and-absolute-references-981f5871-7864-42cc-b3f0-41ffa10cc6fc)

## 1. Purpose
This note defines the custom formula editor that `DnaOneCalc` should build.

It exists to:
1. reduce the risk of the custom-editor decision,
2. pin the Excel compatibility floor that matters for this product,
3. define where OneCalc should deliberately go beyond Excel,
4. keep the editor narrower than a general code editor,
5. and give implementation and tests one explicit reference.

It is not:
1. a generic text-editor framework spec,
2. a commitment to worksheet-grid editing or workbook references,
3. or permission to invent a second parser or language-service truth locally.

## 2. Product Reading
The editor is:
1. a specialized Excel-formula editor,
2. the primary interaction surface for `Explore`,
3. a reusable DNA Calc formula-entry surface for later apps,
4. and a product surface that must integrate tightly with OxFml and OxFunc.

Input-model rule:
1. the edited payload is any text that could be entered into an Excel cell,
2. not only leading-`=` formulas,
3. so the same editor must handle formulas, direct values, and apostrophe-forced strings.

It is not:
1. a generic code IDE,
2. a rich document editor,
3. a collaborative editor,
4. or a styling playground with arbitrary user customization.

## 2A. OxFml Coupling Rule
This editor is intentionally coupled to OxFml.

It should be read as:
1. a specialized OneCalc and DNA Calc editing surface over OxFml syntax and language-service truth,
2. not a generic editor with a replaceable parser plugin,
3. and not a host-local reimplementation of tokenization, trivia ownership, parse trees, or semantic context.

Current OxFml implementation direction already provides:
1. immutable green trees as canonical syntax artifacts,
2. contextual red projections over those green trees,
3. `FormulaEditRequest` and `FormulaEditResult` packet flow,
4. `EditorSyntaxSnapshot` with owned leading and trailing trivia for editor rendering,
5. `LiveDiagnosticSnapshot`,
6. deterministic completion packets,
7. completion validation and application through the ordinary edit path,
8. cursor-sensitive `SignatureHelpContext`,
9. incremental parse, red-projection, and bind reuse summaries.

Working rule:
1. OneCalc should lean into this coupling to simplify the editor,
2. keep generic abstraction only at adapter and test-double boundaries,
3. and treat OxFml as the canonical syntax-tree and editor-language-service authority.

## 3. Scope And Non-Scope
### 3.1 Current Scope
The current editor scope includes:
1. single-cell-entry editing,
2. multi-line structured editing,
3. formula-specific completions,
4. signature help and current help,
5. diagnostics and squiggles,
6. selection-aware editing commands,
7. undo and redo,
8. short and medium formulas as the dominant case,
9. long formulas up to a practical bound of roughly `8000` characters,
10. optional decorations and semantic overlays where admitted,
11. browser and desktop compatibility over one shared `Leptos` core,
12. fallback to a basic but functional text box if advanced behavior cannot be activated.

This current scope explicitly includes:
1. leading-`=` formulas,
2. direct value entry such as `123.4`,
3. apostrophe-forced string entry such as `'123.4`,
4. and Excel-like result plus effective-display behavior for those entries under admitted formatting context.

### 3.2 Immediate Non-Scope
The first serious editor wave does not require:
1. code folding,
2. collaborative editing,
3. multiple carets or selections,
4. arbitrary user-customizable theming,
5. arbitrary syntax plugins,
6. generic language support beyond Excel-formula semantics.

### 3.3 Design-For-Later Scope
The editor should be designed so that later additions remain possible:
1. richer OxFunc-backed help prose,
2. enum and constant pickers,
3. ghost-text suggestions,
4. IntelliCode-like larger formula suggestions,
5. evaluation of selected subexpressions,
6. hover and inspector synchronization,
7. inter-line semantic decoration strips,
8. stronger accessibility, RTL, and bi-directional text support.

## 4. Excel Compatibility Floor
The compatibility target is the Excel formula-entry experience where it is relevant to a single-formula host.
In this note, “formula-entry” includes Excel cell-entry behavior for non-formula values and forced strings as well as leading-`=` formulas.

This note treats Microsoft-documented Excel behaviors as the compatibility floor.
Where Microsoft docs are silent, this note makes OneCalc-owned decisions rather than claiming hidden Excel parity.

Platform priority rule:
1. Windows desktop is the strict-first compatibility target for Excel-style shortcut and editing behavior,
2. browser/WASM is best-effort compatibility where browser and platform constraints permit,
3. browser limitations must degrade honestly rather than pretending exact desktop parity.

### 4.1 Compatibility Behaviors To Preserve
OneCalc should preserve these Excel-adjacent behaviors:
1. leading `=` enters formula mode,
2. direct value entry remains valid cell-entry text,
3. apostrophe-forced string entry remains valid cell-entry text and preserves string intent,
4. formula autocomplete appears from formula context and narrows from typed prefix,
5. `Tab` is the primary completion-accept key,
6. typing a comma during function entry can continue argument-aware help flow,
7. function ScreenTip-style argument help appears after a function name and opening parenthesis,
8. `F2`-style edit-mode behavior is meaningful for formula editing and selection movement,
9. `Ctrl+Shift+U`-style expand and collapse of the formula surface has a OneCalc equivalent,
10. `F4` cycles relative and absolute reference forms when a reference token is selected,
11. `Esc` cancels current entry or operation cleanly,
12. `Enter` commits the current edit in the normal path,
13. `Shift+F3` and `Ctrl+A` style function-argument assistance have a OneCalc equivalent surface.

Shortcut interpretation rule:
1. on Windows desktop, these behaviors should map as closely as practical to Excel expectations,
2. on browser/WASM, equivalent outcomes are required but exact key bindings may vary where the platform reserves or blocks the desktop binding.

### 4.2 Compatibility Behaviors That Must Be Deliberately Reinterpreted
Excel behaviors tied to the worksheet grid must not be copied blindly.

These require deliberate reinterpretation:
1. point mode based on sheet-cell picking,
2. formula-bar versus in-cell duality,
3. workbook named-range insertion tied to worksheet context,
4. grid-relative keyboard movement semantics,
5. range-fill behaviors such as `Ctrl+Enter` over selected cell blocks.

Rule:
1. OneCalc should preserve formula-authoring ergonomics,
2. preserve ordinary cell-entry ergonomics for direct values and forced strings,
3. but not imply workbook-grid scope it does not own.

## 5. OneCalc-Owned Enhancements
OneCalc should go beyond Excel where the enhancement is formula-specific and product-relevant.

The intended enhancement families are:
1. stronger function and argument help during editing,
2. better visibility into support status, preview state, and capability truth,
3. enum and constant selection where OxFunc metadata admits it,
4. semantic hover and selection-driven introspection,
5. decoration layers that can expose semantic status without breaking editing flow,
6. stronger integration with X-Ray and retained evidence.

Rule:
1. enhancements must deepen the formula-authoring surface,
2. not turn the editor into a general IDE.

## 6. Editor Shape Versus Generic Code Editors
This editor should be simpler than a general code editor in important ways.

Why:
1. the edited unit is one formula, not a file,
2. the language is narrower and structurally constrained,
3. most sessions involve short or medium formulas,
4. the product is not optimizing for unbounded text scale.

Derived design consequences:
1. internal structures can be simpler than rope-heavy large-file editors,
2. view-model recomputation can assume smaller text sizes,
3. token and span overlays can be formula-specific rather than language-agnostic,
4. command design can center formula entry and inspection instead of general editing,
5. OneCalc does not need its own generic tokenization or syntax-tree layer because OxFml already owns the immutable syntax substrate.

## 7. Core Editor Requirements
The editor must support:
1. multiline editing,
2. stable caret and selection behavior,
3. internal scrolling for long formulas,
4. `Tab` and `Shift+Tab` indent and outdent with spaces,
5. enter and newline behavior that preserves useful indentation,
6. deterministic completion,
7. signature help,
8. live diagnostics and squiggles,
9. lightweight current help,
10. undo and redo,
11. good paste behavior,
12. keyboard-first flow,
13. result visibility while editing,
14. configurable bracket closing and bracket highlighting,
15. configurable non-standard assistive decorations.

Activation rule:
1. formula-specific assists such as completion, signature help, reference cycling, and argument help should activate only when the current entry meaning makes them relevant,
2. direct value and apostrophe-forced string entry should remain simple and fast, without spurious formula affordances.

## 8. Interaction Surfaces
The editor surface should be designed as several coordinated layers.

### 8.1 Native Input Layer
The simple editing path should use native browser input, selection, and text-edit behavior as the editing authority.

Interpretation:
1. OneCalc should prefer the platform text-input engine for ordinary typing, selection, IME, clipboard, and baseline editing behavior,
2. and should not reimplement text input from raw key events when the platform already does the right thing.

### 8.2 Presentation Layer
Above the native input authority, OneCalc should project:
1. syntax coloration,
2. diagnostics,
3. selection-aware highlights,
4. bracket highlights,
5. ghost text where admitted,
6. semantic decorations,
7. current argument focus,
8. optional inter-line information strips.

### 8.3 Suggestion And Help Layer
The editor should support:
1. completion popup
2. signature-help popup
3. current help card
4. enum or constant picker where admitted
5. future AI-assisted ghost suggestions

### 8.4 Inspect Bridge Layer
The editor should expose a controlled bridge to `Inspect` and X-Ray surfaces, including later:
1. hover-linked semantic information,
2. selection-to-X-Ray synchronization,
3. evaluate-selection requests,
4. replace-selection-with-evaluated-result commands where admitted.

Bridge rule:
1. this bridge should correlate through OxFml spans, green-tree identity, red-context summaries, and later richer node identity where OxFml exposes it,
2. rather than maintaining a separate host-local syntax map.

## 9. Architecture Pattern
The recommended editor architecture is:
1. native text-input authority,
2. OxFml-backed immutable document model,
3. formula-specific state model,
4. overlay and decoration planes,
5. typed command handling,
6. functional fallback mode.

This means:
1. the user-facing editor is not merely a plain textarea,
2. but the underlying input path still relies on native browser editing for the simple path,
3. OxFml owns the syntax tree, syntax snapshot, diagnostics, completion, and signature-help truth,
4. OneCalc projects those packets into an editing UX,
5. and the editor can degrade to a plain functional text box if advanced layers fail or are disabled.

## 10. Internal Subsystems
The editor implementation should have explicit substructure for:
1. text buffer model
2. selection and caret model
3. viewport and line-layout model
4. edit command model
5. undo and redo history model
6. syntax and token snapshot projection
7. diagnostics overlay model
8. completion model
9. signature-help model
10. current-help model
11. decoration model
12. hover and inspect-bridge model
13. configuration model
14. fallback-mode model

The editor should also have an explicit OxFml bridge substructure for:
1. immutable edit request and result handling,
2. `EditorDocument` and `EditorSyntaxSnapshot` ingestion,
3. green-tree-key and text-change-range tracking,
4. reuse-summary tracking for diagnostics and performance visibility,
5. red-context and inspect-bridge correlation,
6. function-help lookup request and response projection.

## 11. OxFml And OxFunc Integration Contract
The editor must consume canonical language-service meaning from OxFml.

That includes:
1. `FormulaEditRequest` and `FormulaEditResult`,
2. `EditorSyntaxSnapshot`,
3. `LiveDiagnosticSnapshot`,
4. deterministic completion proposals,
5. completion validation and proposal application,
6. `SignatureHelpContext`,
7. deterministic function-help lookup subject construction,
8. later `IntelligentCompletionContext`,
9. green-tree-key, text-change-range, and reuse-summary evidence.

Function content truth must come from OxFunc through the admitted upstream path.

Rule:
1. the editor may add presentation and interaction logic,
2. it must not define a second parser, binder, tokenization layer, trivia model, typed-value coercion policy, or semantic help truth locally,
3. interpretation of direct value entry, apostrophe-forced string entry, and effective-display hints remains an OxFml-owned semantic responsibility.

### 11.1 Practical Contract Shape
For the current OneCalc editor, the practical contract should be:
1. OneCalc owns raw entered text, selection, viewport, command dispatch, and visual overlay decisions,
2. OxFml owns immutable syntax artifacts and editor-language-service packets,
3. OneCalc should retain the latest `EditorDocument`-shaped state per formula space rather than decomposing it into ad hoc local mirrors,
4. OneCalc should use `EditorSyntaxSnapshot` as the authoritative token-and-trivia rendering input for syntax coloration and span-aware overlays,
5. OneCalc should use reuse summaries and text-change ranges to keep editing responsive and to support later operation/status surfaces.

In current OxFml-local terms, that means the OneCalc editor should expect to consume and preserve fields equivalent to:
1. source record and entered text,
2. text-change range,
3. `EditorSyntaxSnapshot`,
4. green tree,
5. red projection,
6. optional bound formula,
7. optional semantic plan,
8. live diagnostics,
9. reuse summary.

Projection rule:
1. OneCalc may wrap these in app-specific state and view models,
2. but it should avoid flattening away green-tree identity, red-context information, and reuse evidence too early.

### 11.2 Simplification Rule
Because OneCalc and OxFml are intended to co-evolve:
1. the editor should not introduce a broad generic language-service abstraction just to appear decoupled,
2. adapter boundaries should still exist for tests and host isolation,
3. but the active production path should be allowed to name OxFml packet concepts directly where that reduces duplication and ambiguity.

## 12. Configuration Model
The editor should allow configuration for non-standard assistive behavior.

Configurable families should include:
1. automatic bracket closing
2. bracket pair highlighting
3. ghost-text visibility
4. inter-line semantic decoration visibility
5. hover-driven introspection visibility
6. completion aggressiveness
7. inline versus sidecar help preference where the UI supports both

Rule:
1. core editing correctness must not depend on these toggles,
2. and defaults should be conservative and readable.

## 13. Accessibility And Internationalization Readiness
Accessibility and internationalization are not first-pass completion criteria, but the architecture must not block them.

The editor design should remain ready for:
1. keyboard-only use,
2. screen-reader labeling,
3. visible focus treatment,
4. IME-safe input flow,
5. RTL and bi-directional text handling,
6. high-contrast and reduced-motion modes.

Rule:
1. first implementation may defer full maturity here,
2. but must not hard-code assumptions that make later addition expensive or unsafe.

## 14. Staged Feature Set
### 14.1 Stage A: Minimal Viable Editing Core
Required:
1. native text-input path
2. multiline editing
3. caret and selection
4. internal scrolling
5. undo and redo
6. paste and delete correctness
7. `Tab` and `Shift+Tab` indent and outdent with spaces
8. fallback plain-text mode
9. direct value entry and apostrophe-forced string entry through the same editor surface
10. immutable OxFml edit loop with retained green-tree identity and syntax snapshot projection

### 14.2 Stage B: Excel-Compatibility Floor
Required:
1. completion popup with `Tab` accept
2. function ScreenTip-style argument help
3. bracket matching
4. `F4`-style absolute-reference cycling for selected references
5. `F2`-style edit-mode semantics where meaningful in OneCalc
6. equivalent surface for formula-bar expansion and collapse
7. `Esc` cancel and `Enter` commit behavior
8. OxFml-backed reuse-aware incremental refresh over ordinary typing paths

### 14.3 Stage C: OneCalc Product Enhancements
Required:
1. diagnostics squiggles and readable messages
2. current help sidecar or inline help card
3. support-status cues from OxFunc metadata
4. enum and constant pickers where admitted
5. selection-aware semantic highlighting
6. configurable bracket and decoration behavior
7. inspect-bridge correlation through OxFml spans and red-context summaries

### 14.4 Stage D: Deep OneCalc Assist
Later:
1. selection evaluation and replacement
2. hover-to-X-Ray synchronization
3. inter-line semantic decoration strips
4. richer semantic usage guidance
5. ghost-text suggestions
6. IntelliCode-like larger formula suggestions

## 15. Test Strategy
The editor has the highest TDD burden in the UI stack.

Testing should be split across:
1. pure model tests
2. browser interaction tests
3. service-integration tests with real OxFml packet shapes and fake or pinned upstream providers where needed
4. mode acceptance tests in `Explore`
5. fallback and degradation tests

### 15.1 Pure Model Tests
Cover:
1. text insertion and deletion
2. selection updates
3. indent and outdent behavior
4. undo and redo
5. command routing
6. reference-token range detection for `F4` cycling
7. decoration layering decisions

### 15.2 Browser Interaction Tests
Cover:
1. typing and selection with native input
2. paste behavior
3. IME-safe baseline flow
4. completion popup navigation
5. signature-help updates while typing
6. fallback-mode activation

### 15.3 Integration Tests
Cover:
1. OxFml edit request and response loop
2. `EditorSyntaxSnapshot` token and trivia projection into editor rendering state
3. green-tree-key and text-change-range tracking
4. reuse-summary handling across ordinary edits
5. diagnostic rendering
6. completion proposal rendering and application
7. signature-help projection
8. function-help retrieval
9. mode switch from `Explore` to `Inspect` with selection context preserved where admitted
10. direct value entry and apostrophe-forced string entry interpreted entirely through OxFml-owned result and effective-display responses

### 15.4 Acceptance Tests
The editor must trace back to:
1. `EX-01`
2. `EX-02`
3. `EX-03`
4. `EX-04`
5. `EX-05`
6. `EX-07`
7. `EX-09`
8. `IN-09`

## 16. Excel Research Readout
Microsoft’s official documentation supports the following concrete compatibility anchors:
1. Formula AutoComplete is context-triggered from formula entry and from typed prefixes, and `Tab` is the preferred insertion key.
2. Function ScreenTips appear after a function name and opening parenthesis.
3. `F2` is the core edit-mode key, including reference-selection-oriented behavior in formula editing.
4. `Ctrl+Shift+U` expands and collapses the formula bar.
5. `Ctrl+A` and `Ctrl+Shift+A` provide function-argument assistance when the insertion point is at a function name.
6. `F4` cycles a selected reference through relative, mixed, and absolute forms.
7. `Esc`, `Enter`, and related formula-bar commands are first-class formula-entry behaviors.

The overall Excel model also implies that:
1. not all valid cell entries begin with `=`,
2. direct value entry is a first-class input path,
3. and apostrophe-prefixed entry forces string interpretation.

Official docs do not fully specify every Excel editor quirk.

Interpretation rule:
1. documented behaviors are the compatibility floor,
2. undocumented quirks are not parity commitments,
3. OneCalc may improve on Excel as long as formula-authoring compatibility remains strong and the new behavior is explicit and configurable where needed.

## 17. Immediate Build Guidance
The first implementation slices for the editor should be:
1. buffer, selection, and history model
2. native input integration and fallback mode
3. OxFml document bridge and immutable edit loop
4. command map
5. syntax snapshot rendering path
6. completion and signature-help surfaces
7. diagnostics overlay
8. bracket handling and reference cycling
9. current help
10. inspect bridge hooks
11. later semantic decoration planes

## 18. Derived Next Step
This note should now feed:
1. the editor-specific portion of [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md),
2. the first `ui/editor/` module scaffold,
3. the first editor-focused test tree under `tests/editor/`,
4. and later a narrower implementation note for the native-input-backed overlay architecture.
