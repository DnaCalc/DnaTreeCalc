# DNA OneCalc UX Use Cases

Status: `working_use_case_note`
Date: 2026-04-05
Scope: user-scenario-driven UX use cases for the current `DnaOneCalc` product direction

Companion notes:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md)
2. [APP_UX_BRIEF.md](APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](APP_UX_ARCHITECTURE.md)
4. [APP_UX_SCOPE_FORMALIZATION.md](APP_UX_SCOPE_FORMALIZATION.md)
5. [APP_UX_MODE_VISIBILITY_MAP.md](APP_UX_MODE_VISIBILITY_MAP.md)
6. [APP_UX_PANEL_INVENTORY.md](APP_UX_PANEL_INVENTORY.md)
7. [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md)

## 1. Purpose
This note describes user-scenario-driven use cases for the current OneCalc UX.

It exists to:
1. test the intended UX against concrete user goals,
2. cover the ordered product perspectives `Explore`, `Inspect`, and `Workbench`,
3. expose required attention paths through the shell,
4. expose required keyboard and pointer interaction paths,
5. and give later screen specs a narrative test-of-done layer.

It is not:
1. a pixel spec,
2. a workflow-automation spec,
3. or permission to add product scope beyond the current formalization.

## 2. Reading Rule
Interpret these use cases under current scope.

That means:
1. they may stress current panels and modes,
2. they may expose future seam pressure,
3. but they must not be read as automatic permission to broaden current product scope.

## 3. Personas
The current use cases are written against six practical personas.

### 3.1 Mara, Formula Explorer
1. Primary need: author formulas quickly and confidently
2. Main mode: `Explore`
3. Typical risk: gets lost when help, result, and diagnostics are too far apart

### 3.2 Dev, Spreadsheet Mechanism Investigator
1. Primary need: understand why a formula evaluated the way it did
2. Main mode: `Inspect`
3. Typical risk: sees logs but not structure

### 3.3 Lina, QA And Conformance Operator
1. Primary need: compare OneCalc behavior with Excel or retained evidence
2. Main mode: `Workbench`
3. Typical risk: cannot tell whether a mismatch is real, blocked, or just not observed

### 3.4 Sam, Support And Escalation Engineer
1. Primary need: take a confusing customer formula and prepare a coherent escalation packet
2. Main modes: `Explore`, `Inspect`, `Workbench`
3. Typical risk: loses scenario policy or host truth while moving across modes

### 3.5 Priya, Product And Surface Curator
1. Primary need: inspect support status, function affordances, and edge behavior
2. Main modes: `Explore`, `Inspect`
3. Typical risk: product looks broader than the admitted function floor

### 3.6 Eli, Heavy Keyboard User
1. Primary need: move fast without pointer-first friction
2. Main mode: `Explore`
3. Typical risk: editor quality breaks down on long or structured formulas

## 4. Use Case Format
Each use case records:
1. `Persona`
2. `Goal`
3. `Mode`
4. `Attention Path`
5. `Interaction Path`
6. `Done Signal`
7. `Scope Notes`

Interpretation:
1. `Attention Path` describes what the user’s eyes and attention should naturally hit in order,
2. `Interaction Path` describes expected keyboard and pointer flow,
3. `Done Signal` is the UX-level proof that the task is complete,
4. `Scope Notes` record gates, caveats, or seam pressure without widening scope.

## 5. Explore Use Cases
### EX-01 Unexpected Scalar Result
1. Persona: Mara
2. Goal: determine why a freshly entered formula returns an unexpected scalar value
3. Mode: `Explore`
4. Attention Path:
   1. formula editor,
   2. diagnostics,
   3. result panel,
   4. current help
5. Interaction Path:
   1. type or paste formula,
   2. evaluate,
   3. inspect diagnostics,
   4. compare expected versus actual result,
   5. open help if function meaning is unclear
6. Done Signal:
   1. the user understands whether the issue is syntax, semantics, or expectation mismatch
7. Scope Notes:
   1. no inspect-only mechanism view is required yet, but the path to `Inspect` should be obvious

### EX-02 Very Long Formula Authoring
1. Persona: Eli
2. Goal: enter and navigate a very long multi-line formula without losing orientation
3. Mode: `Explore`
4. Attention Path:
   1. formula editor,
   2. line and structure cues,
   3. diagnostics,
   4. result summary
5. Interaction Path:
   1. paste a long formula,
   2. use scrolling, selection, caret movement, and indentation,
   3. use `Tab` and `Shift+Tab` for spaces indentation,
   4. move through the formula by keyboard without collapsing the editing context
6. Done Signal:
   1. the user can navigate, edit, and evaluate the long formula without the editor becoming unusable
7. Scope Notes:
   1. this is a core editor-quality acceptance case
   2. editor resizing or internal scrolling must remain understandable

### EX-03 Completion-Led Function Discovery
1. Persona: Mara
2. Goal: discover the correct function while partially typing a formula
3. Mode: `Explore`
4. Attention Path:
   1. current cursor position,
   2. completion companion,
   3. current help,
   4. result once accepted
5. Interaction Path:
   1. type partial function name,
   2. navigate completion list by keyboard,
   3. accept one completion,
   4. read current help,
   5. continue authoring
6. Done Signal:
   1. the intended function is inserted and the user understands its basic usage without leaving authoring flow
7. Scope Notes:
   1. richer future OxFunc prose is a hook, not a current requirement

### EX-04 Signature And Argument Guidance
1. Persona: Mara
2. Goal: understand which argument is active and what kind of value it expects
3. Mode: `Explore`
4. Attention Path:
   1. cursor in formula,
   2. signature/help companion,
   3. diagnostics if argument shape is wrong
5. Interaction Path:
   1. enter a multi-argument function,
   2. move through arguments,
   3. inspect current help and argument cueing,
   4. correct any invalid argument
6. Done Signal:
   1. the user can finish the formula with fewer trial-and-error edits
7. Scope Notes:
   1. exact richness of signature payload depends on upstream seams

### EX-05 Array-Aware Exploration
1. Persona: Priya
2. Goal: understand the intermediate array behavior of a formula that returns or uses arrays
3. Mode: `Explore`
4. Attention Path:
   1. formula editor,
   2. result panel,
   3. array preview,
   4. path to `Inspect`
5. Interaction Path:
   1. enter an array-producing formula,
   2. evaluate,
   3. inspect the array preview,
   4. decide whether `Inspect` is needed
6. Done Signal:
   1. the user can tell whether the formula is scalar, array-shaped, or uses intermediate arrays that matter
7. Scope Notes:
   1. array preview is a supporting surface, not a spreadsheet grid

### EX-06 Formatting Truth Check
1. Persona: Priya
2. Goal: check how a result is currently displayed and whether formatting is affecting interpretation
3. Mode: `Explore`
4. Attention Path:
   1. result panel,
   2. effective display summary,
   3. formatting entry point
5. Interaction Path:
   1. evaluate formula,
   2. read effective display summary,
   3. open formatting detail if admitted,
   4. inspect display options
6. Done Signal:
   1. the user knows whether the displayed value matches the underlying result and what formatting is active
7. Scope Notes:
   1. formatting remains `needs_clarification`; this use case is partly a scope-confirmation test

### EX-07 Deterministic Versus Real-Time Policy Check
1. Persona: Sam
2. Goal: confirm whether a scenario is frozen or using live volatile behavior
3. Mode: `Explore`
4. Attention Path:
   1. top context bar scenario policy summary,
   2. formula editor,
   3. result
5. Interaction Path:
   1. inspect scenario policy,
   2. change it if the scenario allows,
   3. re-evaluate,
   4. compare result behavior
6. Done Signal:
   1. the user understands whether time or random-sensitive behavior is deterministic or live
7. Scope Notes:
   1. this is a core scenario-policy case

### EX-08 Multiple Formula Spaces
1. Persona: Mara
2. Goal: keep two or three formula spaces open and switch between them without losing state
3. Mode: `Explore`
4. Attention Path:
   1. left rail formula-space list,
   2. active editor,
   3. result and diagnostics for each active space
5. Interaction Path:
   1. open multiple spaces,
   2. pin one important space,
   3. switch between spaces,
   4. verify that formula text, diagnostics, and result state are preserved per space
6. Done Signal:
   1. switching spaces never feels like abandoning one ephemeral scratchpad for another
7. Scope Notes:
   1. this is a shell-model acceptance case

### EX-09 Invalid Formula Repair
1. Persona: Eli
2. Goal: repair a malformed formula quickly using diagnostics and help without leaving the editor
3. Mode: `Explore`
4. Attention Path:
   1. editor error span,
   2. diagnostics panel,
   3. current help,
   4. result once repaired
5. Interaction Path:
   1. type invalid syntax,
   2. inspect diagnostics,
   3. move cursor to faulty area,
   4. edit and re-evaluate,
   5. confirm successful result
6. Done Signal:
   1. the error is fixed through the main editing loop rather than through a separate troubleshooting surface
7. Scope Notes:
   1. diagnostics must stay spatially tied to the editor

### EX-10 Capture A Run From Explore
1. Persona: Sam
2. Goal: turn a meaningful scenario into retained evidence without first doing a full compare workflow
3. Mode: `Explore`
4. Attention Path:
   1. formula and result,
   2. scenario policy summary,
   3. retain or capture action
5. Interaction Path:
   1. author formula,
   2. evaluate,
   3. confirm scenario policy,
   4. retain the run,
   5. move to `Workbench` if needed
6. Done Signal:
   1. the scenario becomes durable evidence with preserved context
7. Scope Notes:
   1. every meaningful session should be capable of becoming retained evidence

## 6. Inspect Use Cases
### IN-01 Unexpected Inner Function Behavior
1. Persona: Dev
2. Goal: understand why an inner function evaluates in an unexpected way
3. Mode: `Inspect`
4. Attention Path:
   1. formula walk,
   2. relevant subtree,
   3. bind and eval summaries,
   4. source formula summary
5. Interaction Path:
   1. enter `Inspect`,
   2. expand the relevant formula-walk nodes,
   3. inspect subtree behavior,
   4. open node detail if necessary
6. Done Signal:
   1. the user can isolate the suspicious subexpression and explain its apparent behavior
7. Scope Notes:
   1. deeper per-node partial evaluation remains an upstream seam pressure item

### IN-02 Bound Name Investigation
1. Persona: Dev
2. Goal: confirm what a named binding in a `LET`-style formula actually refers to
3. Mode: `Inspect`
4. Attention Path:
   1. formula walk,
   2. bind summary,
   3. selected node detail
5. Interaction Path:
   1. select or expand the binding node,
   2. inspect bind summary,
   3. compare binding target and downstream use
6. Done Signal:
   1. the user understands whether the name binding itself is correct
7. Scope Notes:
   1. this is a core semantic-inspection case

### IN-03 Blocked Or Opaque Reason Check
1. Persona: Dev
2. Goal: determine whether an inspect gap is because something is blocked, opaque, or simply not projected
3. Mode: `Inspect`
4. Attention Path:
   1. formula walk state markers,
   2. provenance summary,
   3. inspect detail drawer
5. Interaction Path:
   1. identify a non-evaluated or unclear node,
   2. inspect its state category,
   3. open detail if needed,
   4. read the reason
6. Done Signal:
   1. the user can explain why full detail is missing without guessing
7. Scope Notes:
   1. this is an honesty-surface acceptance case

### IN-04 Parse Structure Investigation
1. Persona: Priya
2. Goal: inspect whether the formula was parsed in the shape the user intended
3. Mode: `Inspect`
4. Attention Path:
   1. source formula summary,
   2. formula walk,
   3. parse summary
5. Interaction Path:
   1. compare source text to tree structure,
   2. inspect parse summary,
   3. identify whether precedence or grouping is the issue
6. Done Signal:
   1. the user can tell whether the problem is author intent versus parse structure
7. Scope Notes:
   1. parse tree visibility is explicitly in current scope

### IN-05 Evaluation Context Check
1. Persona: Dev
2. Goal: inspect timing, packet kind, or evaluation context when behavior is surprising
3. Mode: `Inspect`
4. Attention Path:
   1. eval summary,
   2. host context,
   3. scenario policy summary
5. Interaction Path:
   1. enter `Inspect`,
   2. read eval summary and host context,
   3. confirm scenario policy,
   4. decide whether the issue is context-driven
6. Done Signal:
   1. the user can distinguish semantic issues from context or host-policy issues
7. Scope Notes:
   1. packet-level detail should stay interpretive, not dump-like

### IN-06 Read-Only Confidence Path
1. Persona: Sam
2. Goal: inspect a customer scenario without accidentally editing the authored formula
3. Mode: `Inspect`
4. Attention Path:
   1. read-only source formula panel,
   2. formula walk,
   3. source result summary
5. Interaction Path:
   1. enter `Inspect`,
   2. inspect read-only panels,
   3. return to `Explore` only if edits are actually needed
6. Done Signal:
   1. the user can perform a semantic review without fear of changing the scenario
7. Scope Notes:
   1. read-only mode discipline matters for support and escalation work

### IN-07 Function Support Status During Inspection
1. Persona: Priya
2. Goal: understand whether a function’s admission or support status explains current behavior
3. Mode: `Inspect`
4. Attention Path:
   1. function-related node,
   2. host context and capability cues,
   3. function guidance if available
5. Interaction Path:
   1. inspect the relevant node,
   2. check support or admission cues,
   3. open extra function detail if available
6. Done Signal:
   1. the user can tell whether the issue is due to support-floor truth rather than pure formula logic
7. Scope Notes:
   1. richer function metadata is partly a future hook

### IN-08 Attention Path For Deep Trees
1. Persona: Dev
2. Goal: stay oriented while exploring a deeply nested formula walk
3. Mode: `Inspect`
4. Attention Path:
   1. top-level result,
   2. major subtrees,
   3. selected inner node,
   4. provenance detail only if required
5. Interaction Path:
   1. expand only the relevant branch,
   2. keep the rest collapsed,
   3. use keyboard or pointer to move focus,
   4. open drawer detail only for the chosen node
6. Done Signal:
   1. the user can work through deep structure without the screen degrading into an unreadable tree wall
7. Scope Notes:
   1. this is a visual hierarchy and attention-management acceptance case

### IN-09 Transition Back To Explore
1. Persona: Mara
2. Goal: discover a semantic problem in `Inspect` and return to `Explore` to fix it quickly
3. Mode: `Inspect` then `Explore`
4. Attention Path:
   1. suspicious node,
   2. source formula summary,
   3. path back to authoring
5. Interaction Path:
   1. inspect the problem,
   2. decide it requires formula changes,
   3. return to `Explore`,
   4. edit and re-evaluate
6. Done Signal:
   1. switching modes feels like staying in the same scenario rather than leaving one tool for another
7. Scope Notes:
   1. this is a one-shell acceptance case

### IN-10 Prepare For Excel Cross-Check
1. Persona: Lina
2. Goal: inspect enough semantic context to justify sending the formula to an Excel-observed comparison on Windows
3. Mode: `Inspect`
4. Attention Path:
   1. formula walk,
   2. host and scenario context,
   3. path to `Workbench`
5. Interaction Path:
   1. inspect suspected anomaly,
   2. confirm scenario policy and host truth,
   3. move to `Workbench` to compare if platform admits it
6. Done Signal:
   1. the user can move to a compare workflow with a clear understanding of what is being tested
7. Scope Notes:
   1. Windows-only Excel comparison must be obvious when admitted

## 7. Workbench Use Cases
### WB-01 Native Excel Comparison For Unexpected Result
1. Persona: Lina
2. Goal: on Windows, compare a suspicious OneCalc result against Excel for the same formula
3. Mode: `Workbench`
4. Attention Path:
   1. comparison outcome,
   2. replay lineage,
   3. platform and capability cues,
   4. blocked dimensions if any
5. Interaction Path:
   1. enter `Workbench`,
   2. trigger or inspect Excel-observed comparison,
   3. read outcome,
   4. inspect lineage and envelope
6. Done Signal:
   1. it is obvious whether the formula matched, differed, or could not be fully compared
7. Scope Notes:
   1. this case is Windows-gated and must be labeled honestly elsewhere

### WB-02 Browser-Only Honest Degradation
1. Persona: Lina
2. Goal: use `Workbench` in a browser-hosted build without being misled about unavailable Excel observation
3. Mode: `Workbench`
4. Attention Path:
   1. workbench mode identity,
   2. capability or platform gate truth,
   3. available replay-only or retained-evidence lanes
5. Interaction Path:
   1. open `Workbench`,
   2. inspect what is available,
   3. understand why Excel-observed compare is absent or disabled
6. Done Signal:
   1. the user understands the current compare floor without overclaim or confusion
7. Scope Notes:
   1. this is a core platform-honesty case

### WB-03 Capture An Anomaly For Escalation
1. Persona: Sam
2. Goal: package a suspicious scenario into a coherent capture file or evidence bundle for handoff
3. Mode: `Workbench`
4. Attention Path:
   1. source run summary,
   2. evidence bundle,
   3. handoff panel,
   4. scenario policy summary
5. Interaction Path:
   1. retain the relevant run,
   2. inspect the evidence bundle summary,
   3. export or hand off the bundle,
   4. preserve scenario policy and host truth
6. Done Signal:
   1. the exported or handed-off artifact is coherent enough to reproduce or escalate the anomaly
7. Scope Notes:
   1. this directly tests OneCalc as a proving host rather than a mere viewer

### WB-04 Replay Lineage Review
1. Persona: Lina
2. Goal: understand how a current comparison relates to earlier retained runs
3. Mode: `Workbench`
4. Attention Path:
   1. replay lineage,
   2. comparison outcome,
   3. source run summary
5. Interaction Path:
   1. select a retained scenario,
   2. inspect lineage,
   3. compare current and earlier runs,
   4. determine whether the anomaly is stable or drifting
6. Done Signal:
   1. the user can explain where the current evidence came from and how it evolved
7. Scope Notes:
   1. replay lineage is first-class product scope

### WB-05 Mismatch Interpretation
1. Persona: Lina
2. Goal: understand what differed between OneCalc and Excel and why that difference matters
3. Mode: `Workbench`
4. Attention Path:
   1. comparison outcome,
   2. blocked dimensions,
   3. observation envelope,
   4. path back to `Inspect` if needed
5. Interaction Path:
   1. inspect mismatch surfaces,
   2. determine which dimensions differ,
   3. judge whether a return to `Inspect` is needed,
   4. prepare next action
6. Done Signal:
   1. the user can distinguish true mismatch from comparison incompleteness
7. Scope Notes:
   1. “mismatch meaning” is core workbench value

### WB-06 Noisy Or Partial Observation
1. Persona: Sam
2. Goal: understand whether the comparison is partial, lossy, or blocked before escalating it
3. Mode: `Workbench`
4. Attention Path:
   1. observation envelope summary,
   2. blocked dimensions,
   3. reliability summary,
   4. handoff panel
5. Interaction Path:
   1. inspect envelope and blocked-dimension surfaces,
   2. decide whether more evidence is needed,
   3. widen or hand off accordingly if admitted
6. Done Signal:
   1. the user knows whether the current evidence is escalation-grade
7. Scope Notes:
   1. this is an honesty and evidence-quality case

### WB-07 Retain A Known-Good Witness
1. Persona: Lina
2. Goal: keep a known-good scenario as durable reference evidence
3. Mode: `Workbench`
4. Attention Path:
   1. comparison outcome,
   2. replay lineage,
   3. evidence bundle state
5. Interaction Path:
   1. inspect a good comparison,
   2. retain it,
   3. verify the evidence bundle identity
6. Done Signal:
   1. the known-good scenario can later be reopened as a trusted reference point
7. Scope Notes:
   1. this supports the durable scenario library thesis

### WB-08 Escalation Packet With Context Preservation
1. Persona: Sam
2. Goal: prepare a handoff that preserves formula, result, scenario policy, host truth, and comparison outcome together
3. Mode: `Workbench`
4. Attention Path:
   1. source formula summary,
   2. scenario policy summary,
   3. evidence bundle,
   4. handoff action
5. Interaction Path:
   1. inspect the bundle summary,
   2. confirm context preservation,
   3. execute handoff
6. Done Signal:
   1. the handoff packet contains the minimum coherent evidence needed for upstream follow-up
7. Scope Notes:
   1. this is a direct proving-host acceptance case

### WB-09 Compare After Semantic Investigation
1. Persona: Dev
2. Goal: move from a semantic suspicion in `Inspect` to a compare workflow that answers whether Excel behaves differently
3. Mode: `Inspect` then `Workbench`
4. Attention Path:
   1. suspicious inspect finding,
   2. workbench entry,
   3. comparison outcome,
   4. replay lineage
5. Interaction Path:
   1. inspect the suspicious behavior,
   2. enter `Workbench`,
   3. compare if admitted,
   4. interpret the outcome
6. Done Signal:
   1. the user can bridge mechanism understanding and empirical comparison in one shell
7. Scope Notes:
   1. the UX should make this transition obvious on Windows and honest elsewhere

### WB-10 Decide Next Action
1. Persona: Lina
2. Goal: end a workbench session with a clear next step rather than ambiguous evidence
3. Mode: `Workbench`
4. Attention Path:
   1. comparison outcome,
   2. reliability summary,
   3. blocked dimensions,
   4. handoff or retain action
5. Interaction Path:
   1. inspect the current evidence quality,
   2. choose retain, replay, compare again, export, or handoff,
   3. commit the action
6. Done Signal:
   1. the workbench session ends in a clear retained state or explicit next action
7. Scope Notes:
   1. this is the primary closure case for `Workbench`

## 8. Cross-Correlation Intent
These use cases are meant to be cross-correlated next with:
1. modes,
2. panels,
3. visible surfaces,
4. mutable surfaces,
5. and platform gates.

The next useful derivation from this note is:
1. a panel-to-use-case correlation table,
2. or a first constrained `Explore` screen spec validated against the `EX-*` cases.

Current derived artifact:
1. [APP_UX_USE_CASE_CROSSWALK.md](APP_UX_USE_CASE_CROSSWALK.md)
