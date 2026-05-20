# DNA TreeCalc — Core Model Spec

## 1. Purpose and Positioning

DNA TreeCalc is the first serious multi-node host built on the OxCalc substrate, in the same way DNA OneCalc is the single-formula host built on OxFml. It is not Excel — it has no grid, no coordinates, no spilling between cells, and no `A1:B5` range references. It is a tree of named nodes, each carrying its own formula and its own value, with references between nodes resolved through tree paths.

Layering (per-product repos are authoritative):

| Layer | Owns |
|---|---|
| OxFunc | Value universe (`EvalValue`), function/operator semantics, coercion, error algebra, array lifting |
| OxFml | Grammar, parse, bind, single-node evaluator, format-code parsing, LAMBDA/LET, completion/signature-help |
| OxCalc | Multi-node coordinator, dependency graph, invalidation closure, atomic publication, epochs, tree-substrate types (`TreeNodeId`, `TreeReference`) |
| DNA TreeCalc | User-facing host — workspace persistence, UI, structural editing, cross-workspace orchestration |

The OxCalc substrate uses tree-node-id semantics (not coordinates) and exposes the reference variants `DirectNode`, `RelativePath`, `SiblingOffset`, `DynamicResolved`, `ProjectionPath`, `Unresolved`. TreeCalc is the host on top of an already-tree-shaped engine; this document describes the user-facing surface the host provides.

## 2. Core Model

A workspace is a tree of named nodes. Each node has:

- A **name** (case-insensitive at lookup, display preserves user casing). Sibling names are unique case-insensitively; a node cannot have two regular children, two meta children, or one regular and one meta child whose names differ only by case.
- A **position** among its siblings (`sibling_index`, user-reorderable). Sibling order is stable, persisted, and user-visible; it is the semantic order used by ordered reference collections.
- A **formula** — the node's single content field (OxFml source text). Per Excel's cell-entry convention, a leading `=` introduces a formula; an empty string is an empty node; any other entry is a **literal constant** that OxFml parses and resolves to a typed value (§6). A leading apostrophe (`'`) forces text. There is no separate "literal value" content kind; a constant (including an array literal) is just a `formula` without a leading `=`.
- A computed / observable **value** (`Empty` or `EvalValue` — scalar, array, error, reference, lambda).
- A **format** (full per-node formatting, DnaOneCalc-style: number format, font, fill, conditional format rules, data bars, icon sets).
- Zero or more **child nodes**.

A non-leaf node is not implicitly its children's values. Every node has its own formula→value pair regardless of whether it has children. To get a non-leaf's children as a collection, use the explicit children accessor `.@CHILDREN` (sugar: `.*`).

Every node carries its own (formula, value, format) regardless of how many children it has. There is no leaf/non-leaf type distinction at the structural level — "leaf-shaped" is a UI concept (no children) but not a structural one.

Each node carries an `is_meta` boolean attribute (default `false`). When `is_meta` is true on any node, that node and its entire descendant subtree are **invisible to formula reference resolution** and **not bound or evaluated** by the engine. Formula language has no syntax for reaching meta-nodes; they exist as host-managed tree data (templates, formatting, etc.). See [`META_NODES.md`](META_NODES.md).

**Workspace ≈ Excel workbook.** The workspace root is the unnamed top container, persisted as a workspace file. By convention (not enforced):
- Top-level children of root act as "sheets".
- Children of a "sheet" act as defined-names-in-sheet.
- Further nesting is free-form; users can simulate grids by naming children "A1"/"A2"/... or "A"/"B" with "1"/"2" sub-children. Names are just strings; the resemblance is cosmetic.

**No range references.** TreeCalc has no `:` operator over peer nodes. Multi-value lives on a single node (as a dynamic-array value) or in a children-collection (via `.*`).

**No inter-node spilling.** A node's array value stays on that node; it never overwrites adjacent nodes. Dynamic arrays at the node level are fully supported — only Excel-style grid spill across cells is excluded.

## 3. Reference Syntax

The user-facing path syntax for references between nodes.

### 3.1 Separators and anchors

`.` is the segment separator in paths.

`!` is accepted as a separator alias **only at one position**: immediately after the first identifier of a path (the "sheet position"). `Sheet1!Foo.Bar` ≡ `Sheet1.Foo.Bar`. Mid-path `!` (e.g., `Sheet1.Foo!Bar`) is rejected. Leading `!` is rejected. This narrow allowance preserves Excel's `Sheet!Name` and `Sheet!Cell` conventions for both TreeCalc-mode import and future DNA Calc grid-mode use, where `!` after a sheet name is the canonical sheet-to-content separator.

Three anchor sigils, each with one role:

- `^` (and stacks `^^`, `^^^`, ...) — parent / fixed-depth ancestor of the caller. Single reference.
- `[<workspace>]` — workspace selector. Empty brackets `[]` = the current workspace; named or pathed brackets `[ws]` / `['C:\...']` = another workspace. Brackets always appear at the start of an expression and always anchor an absolute path from that workspace's root.
- Bare name — unanchored lexical walk-up lookup. Walk-up resolution skips meta-flagged nodes and their descendants; they are not reachable from formulas.

### 3.2 Path forms

```
Margin                            lexical walk-up lookup of "Margin" from the caller's scope
Q1.Margin                         lexical walk-up lookup of "Q1", then its child "Margin"
^                                 caller's parent (single reference)
^.Margin                          parent's child "Margin"
^^.Total                          grandparent's child "Total"
^^^                               great-grandparent
[]                                the workspace root anchor (navigation only; naked value use errors)
[]Sheet1.Margin                   workspace-rooted absolute path
[].@CHILDREN                      workspace root's children (all top-level "sheets")
[][Sales Q1].Margin               workspace-rooted path whose first segment needs escaping
[ws]Branch1.MyNode                cross-workspace reference (see §3.3)
ref.@NAME                         meta-accessor: node's display name (see §3.5)
ref.*                             children-as-references collection (sugar for ref.@CHILDREN)
@PREV.Net                         previous sibling's Net
@NEXT.Margin                      next sibling's Margin
```

**Bare names resolve by lexical walk-up.** Search caller's own children first, then each ancestor's children one scope at a time (parent's children, grandparent's children, ..., up to root's children). First match wins. If no match in any scope, the reference is `Unresolved`. This generalizes Excel's nearest-scope-wins rule (sheet scope, then workbook scope) from two levels to arbitrary tree depth.

Consequences:
- An ancestor is findable by its own name (`Q1` from inside `Q1.Income.Sales` resolves to `Q1` as a child of `2005`).
- Self-reference (a node's formula writing its own name) resolves to self; under the default (non-iterative) profile this produces a circular-reference error, and with iterative calculation enabled it participates in the iterated cycle region (see §7a).

To force a specific resolution and override the walk-up, the user writes an explicit relative (`^.Name`, `^^.Name`), a deeper descent (`Q1.Margin`), or a workspace-rooted (`[]Name`) or cross-workspace (`[ws]Name`) absolute. The editor surfaces the resolved binding on hover so shadowing is visible at write-time.

### 3.3 Cross-workspace references and bracket-position disambiguation

`[<workspace>]` is the workspace selector at the start of an expression. Empty brackets `[]` mean the current workspace; bracket content identifies another workspace by alias or by path.

```
[]Sheet1.Margin                                      current workspace, from root
[projections]Branch1.MyNode                          alias from workspace manifest
['C:\Work\side-projections.dnatree']Branch1.MyNode   quoted direct path
[projections.dnatree]Branch1.MyNode                  bare direct path
[ws][Branch X].MyNode                                cross-workspace + bracket-escaped first segment
```

Aliases are registered in a section of the workspace persistence file mapping short names to external workspace files. Direct paths work for ad-hoc references; the UI offers "promote to alias". A bracket prefix is always absolute from the selected workspace's root — `^` (up-step) is meaningless after a bracket selector and is rejected by the parser. The default is live-latest: a cross-workspace reference reads whatever the external workspace currently publishes.

Engine mapping: bind layer produces an `External(ExternalRef)` carrier (OxFml). From OxCalc-Tree's view, this is a `HostSensitive` reference. The host adapter loads external workspaces, caches their published values, and signals invalidation on external republish.

**Bracket-position resolution.** Brackets do several jobs in TreeCalc; position and content determine which:

| Position | Content | Meaning |
|---|---|---|
| Start of expression | `[]` | current-workspace root anchor |
| Start of expression | `[@Name]` | implicit current-row column (column-formula context only) |
| Start of expression | `['quoted path']` | workspace selector (always) |
| Start of expression | `[Word]` / `[Word with spaces]` | workspace selector if the content matches a registered alias; otherwise bare-escaped node name resolved via walk-up |
| After a path, no separator | `path[Col]` | table column ref (LHS Table-typed) |
| After a path, no separator | `path[#Spec]` | structured-reference special item on a Table-typed LHS |
| After a path, no separator | `path[@Col]` | implicit current-row column (column-formula context) |
| After a `.` separator | `path.[Name]` | descent to a node whose name needs escaping |
| Double-bracket form | `path[[Sub]:[Sub2]]`, `path[[#Headers],[Col]]` | structured-ref composite per Excel rules |

The start-of-expression `[Word]` ambiguity (workspace alias vs. bare-escaped first path segment) is resolved at bind time: the manifest is consulted first, and a bare-escaped name lookup is the fallback. If both meanings are possible, the alias wins, and the editor must surface the binding on hover plus a collision warning. Alias registration should reject or warn on collisions with root-level node names, and the user can disambiguate a node by writing an explicit anchor (`[][Name]`, `^.[Name]`, `^^.[Name]`, etc.) when the intended scope is known.

**Bracket design notes / open improvement.**

Excel does not provide an escape form for defined names containing spaces because those names are invalid: Microsoft documents that defined names cannot contain spaces and suggests underscore or period as word separators. Excel uses brackets in adjacent surfaces instead:
- structured references use brackets (and sometimes double brackets) around table column names and special items;
- linked data type fields use the dot operator, and Excel automatically adds brackets around field names that contain spaces, e.g. `=A2.[52 Week High]`.

TreeCalc currently borrows both ideas: bracketed node segments for path names with spaces, and bracketed selectors for structured/table/data-type style access. The cleanest long-term rule may be stricter than the current alias-first fallback: reserve start-of-expression brackets for workspace selectors only, and require escaped first path segments to be anchored explicitly (`[][Sales Q1]`, `^.[Sales Q1]`) or to appear after a dot. That would reduce ambiguity but make walk-up lookup of an escaped first segment less terse. Decide this before parser lock; until then, keep alias-first fallback, mandatory hover disclosure, and collision warnings.

Engine mapping: bind layer produces an `External(ExternalRef)` carrier (OxFml). From OxCalc-Tree's view, this is a `HostSensitive` reference. The host adapter loads external workspaces, caches their published values, and signals invalidation on external republish.

### 3.4 Node names and bracket escaping

TreeCalc borrows Excel structured-reference escaping for bracketed names (Microsoft's [structured-references doc](https://support.microsoft.com/en-au/office/using-structured-references-with-excel-tables-f5ed2452-2337-4f71-bed3-c8ae6d2b276e)), but the unescaped path identifier is TreeCalc-specific:

- **Bare identifier** (no escape required): `[A-Za-z_][A-Za-z0-9_\\]*` — letters, digits (not leading), underscore, backslash. This is intentionally close to Excel defined-name spelling where useful, but it is not Excel's full defined-name grammar. In particular, dots are NOT in the bare identifier set because `.` is always the TreeCalc path separator. Import/export handle this deliberately: dotted Excel names unroll to paths on import, and paths flatten back to dotted names on export where the workspace stays within the Excel-defined-name subset.
- **Bracket-escaped name** (required when the name contains any character outside the bare set): `[Sales Q1]`, `[Net Revenue]`, `[2025-Q1]`, `[$Forecast]`.
- **Reserved characters** that need an inner single-quote escape: `[`, `]`, `#`, `'`, `@`. Verbatim from Excel — the same five chars.
  - `[Foo'[Bar]` — name literally `Foo[Bar`
  - `[Foo']Bar]` — name literally `Foo]Bar`
  - `[Col'#Name]` — name literally `Col#Name`
  - `[Foo''Bar]` — name literally `Foo'Bar` (apostrophe doubled per Excel rule)
  - `['@Special]` — name starting literally with `@`
- **Case:** insensitive at lookup, display preserves user casing.

Bracket escaping applies uniformly: tree-path segments, table column names, cross-workspace path segments, and structured-ref selectors all use the same rule.

### 3.5 Meta-accessor sigil `@`

`@`-prefixed identifiers access metadata about a node rather than descending into it. Capitalization is UPPERCASE to match Excel function-name convention.

Two surface forms:

- **Postfix accessor on a reference:** `ref.@FOO` — gives the meta property of `ref`.
- **Free-standing self-anchored navigator:** `@FOO` — implicitly anchored on the calling node, analogous to bare `^` for "this node's parent".

The family:

| Accessor | Free-standing form | Postfix on `ref` | Result kind |
|---|---|---|---|
| `@NAME` | `@NAME` (caller's name) | `ref.@NAME` | text |
| `@FORMULA` | `@FORMULA` | `ref.@FORMULA` | text (source formula) |
| `@INDEX` | `@INDEX` | `ref.@INDEX` | number (1-based sibling position among regular siblings; meta siblings skipped) |
| `@PARENT` | (`^` is the shorthand) | `ref.@PARENT` | single ref (`#REF!` at workspace root) |
| `@CHILDREN` | (`.*` is the shorthand on this-node, but rare) | `ref.@CHILDREN` (= `ref.*`) | set of refs |
| `@PREV` | `@PREV` | `ref.@PREV` | single ref (`#REF!` on out-of-range) |
| `@NEXT` | `@NEXT` | `ref.@NEXT` | single ref (`#REF!` on out-of-range) |
| `@PRECEDING` | `@PRECEDING` | `ref.@PRECEDING` | set of refs (earlier siblings, sibling-index order) |
| `@FOLLOWING` | `@FOLLOWING` | `ref.@FOLLOWING` | set of refs (later siblings, sibling-index order) |
| `@ANCESTORS` | `@ANCESTORS` | `ref.@ANCESTORS` | set of refs (closest first, up to root) |

Free-standing forms are sugar for `THIS.@FOO` where `THIS` is the implicit calling node. TreeCalc does not expose a `THIS` token; the free-standing form is always available where applicable.

### 3.5b Navigation principle

Path-navigation operators are either **deterministic (single result)** or **set-producing (collection)**. No operator implicitly searches multiple candidate positions and picks one. Closest-match / first-match selection is always explicit composition via `FILTER` + `INDEX`.

| Operator | Single or set? |
|---|---|
| Bare name, descent (`Foo.Bar`), workspace anchor (`[]Foo`, `[ws]Foo`) | single |
| Parent (`^`), fixed-depth ancestor (`^^`, `^^^`, ...), workspace root (`[]` alone) | single navigation ref (`[]` errors if evaluated naked) |
| `@PREV`, `@NEXT`, `@PARENT` | single |
| `@NAME`, `@INDEX`, `@FORMULA` | single (value, not ref) |
| `.*` / `@CHILDREN`, `@ANCESTORS`, `@PRECEDING`, `@FOLLOWING`, `**` | set |

Example — "closest enclosing Year" is explicit composition, not a path-level operator:

```
INDEX(FILTER(@ANCESTORS, a -> a.@NAME = "Year"), 1)
```

**Collection order and duplicates.** "Set-producing" means "collection-producing"; these are ordered reference collections, not mathematical sets.

- `.*` / `@CHILDREN` returns regular children in sibling order, excluding meta-effective children.
- `@PRECEDING` returns earlier regular siblings in ascending sibling order; `@FOLLOWING` returns later regular siblings in ascending sibling order.
- `@ANCESTORS` returns closest ancestor first, then outward toward the workspace root.
- `**` returns descendants in stable depth-first pre-order under the base node, excluding meta-effective subtrees. `Foo.**.Bar` filters that traversal to matching `Bar` descendants while preserving traversal order.
- Explicit reference-array literals preserve source order. Duplicate references are preserved in explicit literals; navigation-produced collections do not duplicate a node because the tree traversal visits each node once.
- Empty collections are valid reference collections. Single-reference navigators such as `@PREV`, `@NEXT`, `^`, or `@PARENT` do not become empty collections on failure; they produce `#REF!`.

### 3.6 Recursive descent `**`

`**` is the any-depth descent operator. `Foo.**.Bar` produces the set of every `Bar` node at any depth under `Foo`. Like all set-producing operators, it returns a reference collection; selection from it is via `INDEX`/`FILTER`/`COUNT`.

```
Accounts.2005.**.Margin           every Margin at any depth under Accounts.2005
Accounts.**                       every descendant of Accounts (collection)
**.Margin                         every Margin in the workspace (anchored to caller)
Sheet1.**                         every descendant of Sheet1
```

### 3.7 Reference-to-engine mapping

| Surface | Engine `TreeReference` variant |
|---|---|
| `Margin` (bare) | Resolved at bind time by lexical walk-up. Result is one of: `RelativePath { base: SelfNode, path: ["Margin"] }` (own-child match), `RelativePath { base: ParentNode, path: ["Margin"] }` (sibling-level match), or `RelativePath { base: Ancestor(n), path: ["Margin"] }` for n ≥ 2 (deeper enclosing scope). If no scope matches, `Unresolved`. |
| `Q1.Margin` (bare descent) | `Q1` resolves by walk-up; descent into the resolved node's child `Margin`. Result includes the same base-variant choice as a bare lookup. |
| `^` | `RelativePath { base: ParentNode, path: [] }` |
| `^.Margin` | `RelativePath { base: ParentNode, path: ["Margin"] }` |
| `^^.Total` | `RelativePath { base: Ancestor(2), path: ["Total"] }` |
| `[]Sheet1.Margin` | `ProjectionPath { projection_path: "Sheet1.Margin" }` |
| `[]` alone | workspace root anchor / ref for further navigation; evaluating it as a value without an accessor or path tail produces `#REF!` |
| `[ws]Branch1.MyNode` | OxFml `External` → OxCalc `HostSensitive` |
| `@PREV.Net` | `SiblingOffset { offset: -1, tail: ["Net"] }` |
| `@NEXT` | `SiblingOffset { offset: +1, tail: [] }` |
| `.*` / `ref.@CHILDREN` | children-set reference (new carrier — see §6) |
| `@ANCESTORS`, `@PRECEDING`, `@FOLLOWING`, `**` | set-membership references (new carrier — see §6) |
| `ref.@NAME` / `@INDEX` / `@FORMULA` | text/number values resolved at evaluation, not structural refs |

## 4. Capability Profile

Every TreeCalc-specific extension to the formula language is gated by a named profile carried in `OxCalcTreeHostCapabilitySnapshot.capability_profile_id` (the existing capability-snapshot field in `oxcalc-core/src/consumer.rs`). Under a strict-Excel profile, the engine rejects all TreeCalc syntax at parse/bind time.

Two relevant profile values:

- `"host-capabilities:strict-excel"` — strict Excel surface. Default for grid hosts (DNA Calc and successors). A formula like `={A1, B2}` is rejected.
- `"host-capabilities:treecalc-v1"` — DNA TreeCalc surface. Enables all the extensions enumerated below. Default for TreeCalc.

The profile is a named, versioned bundle. Future profiles (`treecalc-v2`, etc.) can add features. A workspace persistence file records the profile it was authored under; opening a file under a profile the host doesn't recognize is a hard error, not a silent partial-load.

**What `treecalc-v1` enables** (everything in this list is parse/bind-time-rejected under strict-excel):

- Tree-path syntax: `.` separator, `^` up-step and stacks, fixed-depth ancestor descent, `[]`/`[ws]` workspace selectors, bracket-escaped name segments `[Sales Q1]`
- Meta-accessor `@`-sigil family in both forms (free-standing and postfix): `@NAME`, `@FORMULA`, `@INDEX`, `@PARENT`, `@CHILDREN`, `@PREV`, `@NEXT`, `@PRECEDING`, `@FOLLOWING`, `@ANCESTORS`
- Children-as-collection `.*` (equivalent to `@CHILDREN`)
- Recursive descent `**`
- Reference-array literals `{Foo, Bar}`
- Bare-name lexical walk-up scope rule (replaces Excel's defined-name lookup)
- `INDIRECT(string)` parsing the string as a tree path (under strict-excel, the same function parses as A1/R1C1)

**What stays profile-agnostic** (identical behavior under any profile):

- All function semantics, operators, value coercion (OxFunc)
- Number / text / logical / error values, number formats, conditional formats
- LAMBDA / LET and other functional constructs
- Scalar array literals `{1, 2, 3}`

Obligations on each layer:

- OxFml parser/binder consults `capability_profile_id` for profile-gated grammar and lookup rules.
- OxFunc remains profile-agnostic: function semantics are identical under any profile. The profile gates the *input surface* (which references and literals can be written) and the parsing of string-valued arguments to `INDIRECT`. What a function does, once it has its inputs, is fixed.
- OxCalc honors the profile when integrating bind artifacts: only the reference variants admissible under the profile appear in dependency graphs.
- Capability mismatch is a bind-time error with a clear message — never a runtime error and never silent semantic drift.
- Workspace persistence records the file's authored profile id.

## 5. Excel Alignment Principle

Beyond the tree/node structure and the reference-identifier surface, align with Excel.

| Novel surface (TreeCalc-specific) | Excel-aligned (default to Excel behavior) |
|---|---|
| Tree structure and node identity | Function semantics, operators, value coercion |
| Reference path syntax (`.` separator, `^`, `[]` and `[ws]`, `.*`, `**`) | Number/text/logical/error value behavior; error codes |
| Meta-accessor `@`-family | Number formats, conditional formats, data bars, icon sets |
| Per-node formula+value pair (no grid, no spill) | Date/time/duration handling |
| Workspace as workbook-analog persistence file | Function-name capitalization (UPPERCASE) |
| Rename/move propagation prompts | Array semantics where they coincide (1D/2D arrays of scalar cells) |
| Rich-values theme (rich errors etc.) | LAMBDA, LET, and other functional constructs |
| Single `formula` content field per node | Cell-entry classification: leading `=` = formula, `'` = forced text, else a typed constant (Excel value-entry rules) |

Most design questions have a fast answer: what does Excel do? Capitalization (`@NAME` not `@name`), single-quoting (`'Sales Q1'` not backtick), case-insensitive name lookup, error-code shapes (`#REF!`, `#VALUE!`, `#CALC!`) all follow from this.

## 6. Values, Arrays, and Engine Prerequisites

**Per-node observable value is either `Empty` or an `EvalValue`.** `Empty` is the value of a node whose formula text is the empty string. It is not a formula-evaluation result: no `=...` formula, array formula, or non-empty literal entry can evaluate a node to top-level `Empty`. Formula output may be an empty string (`""`), which is a text value and is distinct from `Empty`. Dereferencing a reference to an empty node can return the `Empty` value to a caller, matching Excel's blank-cell distinction.

The non-empty `EvalValue` domain is scalar `Number`/`Text`/`Logical`/`Error`, plus `Array` (a 2D grid of scalar `ArrayCellValue` cells, including blank cells), `Reference(ReferenceLike)`, and `Lambda`.

**Node content is a single `formula` field; literal constants are unprefixed entries.** Following Excel's cell-entry convention, the entry text is classified before formula parsing: the empty string means the node is empty; a leading `=` is a formula; a leading apostrophe (`'`) forces text (the apostrophe is an entry escape, not part of the value); an unprefixed entry that parses as a finite number is a number constant; `TRUE`/`FALSE` (case-insensitive) is a logical constant; a quoted string is a text constant; any other unprefixed non-empty entry is text preserved verbatim. OxFml already owns the non-empty classification for its single-cell host path (see `OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md` §2.1A); TreeCalc adopts the same rules with the explicit empty-string case above, so `123.4`, `TRUE`, and `=A.B+1` are all just `formula` strings and the leading-`=` discriminator decides constant vs. computed. Consequently `Margin` typed without `=` is the text constant "Margin", while `=Margin` is a reference — exactly as in Excel. Deleting a node's formula/content sets `formula` to `""`, and the node's value becomes `Empty`.

**Dynamic arrays are first-class at every layer.** A formula result can be an array; a literal-constant entry can be an array literal (resized as edited); references to an array-valued node return the whole array. Array cells may be numbers, text, logicals, errors, or empty cells; array cells may not themselves be references, lambdas, or nested arrays.

**Nested arrays are rejected.** Operations that would produce an `EvalValue::Array` whose cells are themselves arrays — for example `MAP(Sheet1.*, c -> c.Margin)` when any Margin's value is itself an array — error out at bind or runtime with `#CALC!`.

**Reference collections are References, not Arrays.** A tree-reference collection (`.*`, `@CHILDREN`, `@ANCESTORS`, `@PRECEDING`, `@FOLLOWING`, `Foo.**`, and explicit `{Foo, Bar}` literals) is carried in `EvalValue::Reference(ReferenceLike)` — the same broad value shape Excel uses for range references like `A1:B5`. TreeCalc does not support grid ranges such as `A1:B5`, but it deliberately creates opaque tree-reference arrays so the upstream libraries are forced to solve the shared abstraction problem once: Excel grid references/ranges and TreeCalc opaque reference arrays should pass through OxFml/OxFunc/OxCalc as uniformly as the model can honestly support.

**Reference abstraction design area (TBD).** This is not expected to have a trivial local answer in TreeCalc. The target behavior is: when a function is marked as not needing references, OxFml/OxFunc can dereference a grid range or tree-reference collection and pass the function an array/value collection; when a function is marked as reference-sensitive, it receives an opaque reference/range-like value and can perform the same class of operations on an Excel range and a TreeCalc reference array where that makes semantic sense. Some Excel range behaviors may not abstract cleanly to unordered or non-rectangular tree sets; those limits must be made explicit in OxFml/OxFunc/OxCalc design, with Excel always kept as the comparison anchor. TreeCalc's role is to expose this pressure early through its reference-array surface, not to paper over it with host-specific per-function behavior.

The following pieces are TreeCalc-specific extensions that depend on engine work in OxCalc / OxFml / OxFunc:

1. **Unified `ReferenceKind` / reference-view abstraction** in OxFunc/OxFml/OxCalc: tree single-node, tree node-set (children/ancestors/preceding/following/descendants/explicit), and tree-dynamic (runtime-resolved via INDIRECT) must coexist with Excel grid references/ranges behind an abstraction that supports both "dereference to values before calling the function" and "preserve opaque reference/range identity for reference-sensitive functions." Iteration logic should plug into shared per-kind dispatch where possible, but the exact abstraction boundary is a design area, not a solved TreeCalc-local detail.
2. **`SelfNode` base variant** in OxCalc's `RelativePath` (or `Ancestor(0)` generalization) so the walk-up scope resolution can capture own-child matches.
3. **Set-membership dependency edge type** in OxCalc's `DependencyGraph`: "this formula depends on the set of children of node X" must invalidate when the set's membership changes structurally (add / remove / rename of a matching child). Same machinery covers `@ANCESTORS`, `@PRECEDING`/`@FOLLOWING`, and `**`.
4. **Reference-array literals** `{Foo, Bar}` — OxFml binder grammar admits references inside `{...}` (not only scalars) under the `treecalc-v1` profile.
5. **Cross-workspace orchestration** via `HostSensitive` — the host adapter loads external workspaces, caches their published values, and signals invalidation on external republish.
6. **Structural-edit semantics** — node rename / move / formula replacement / add / remove: when does each force rebind vs. recalc vs. publication-visible structural delta. Specified in `OxCalc/docs/spec/core-engine/CORE_ENGINE_TREECALC_SEMANTIC_COMPLETION_PLAN.md`. The rename-propagation prompt UX depends on the engine's resolution.
7. **Profile-aware `INDIRECT`** — string argument parsed as a tree path under `treecalc-v1`, as A1/R1C1 under `strict-excel`. Function lives in OxFunc; parsing dispatches on the active profile.
8. **Transactional batch structural editing** in OxCalc — `begin / N edits / commit-or-rollback` atomically as one publication candidate. Useful broadly (multi-node refactors, paste, undo) and specifically needed for clean template sync. Queued as a fundamental engine feature.
9. **`is_meta` per-node attribute** (boolean, default false). When true on any node, that node and its entire descendant subtree are invisible to formula reference resolution AND are not bound or evaluated by the engine. Bind layer skips meta-flagged nodes and their descendants when resolving references (resulting in `Unresolved` on lookup failure). Positional operators (`@PREV`/`@NEXT`/`@INDEX`/`.*`) on regular siblings skip meta neighbors. Templates and per-node format both use this flag; future meta-node uses (configuration, named lambdas, annotations) inherit the same mechanism. Storage / filter strategy is OxCalc's choice.
10. **Conditional-formatting rule semantics for the format surface** — confirm or add support for ordered multiple rules, `Stop If True`, action accumulation across rules, and subtree-level format inheritance inputs. The format engine details live in OxFml/OxFunc, but TreeCalc's format editor and Excel verification depend on this behavior being explicit and reusable rather than reconstructed in the host.
11. **Constant entry classification on the TreeCalc channel** — OxFml's cell-entry classification (`OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md` §2.1A: leading `=` formula, `'` text escape, finite-number/`TRUE`-`FALSE`/quoted-string/verbatim-text constant) must be reachable from the TreeCalc host channel, with TreeCalc's explicit empty-string → `Empty` case and the formula branch parsing tree-path references under `treecalc-v1` rather than WorksheetA1, plus Excel-aligned implicit number-format inference on number constants. Raised in `docs/handovers/HANDOVER_OXFML_constant_input.md`.
12. **Circular-reference cycle profiles** — the host selects a cycle profile and supplies iterative bounds (Maximum Iterations, Maximum Change) via the compatibility basis; OxCalc owns the profiles and the iteration (`docs/spec/core-engine/w048-cycles/`): `cycle.non_iterative_stage1` (default — reject / `cycle_blocked`), `cycle.excel_match_iterative` (Excel-faithful for the current single-host-scoped covered surface), `cycle.iterative_deterministic_v0` (deterministic Jacobi). See §7a and `docs/handovers/HANDOVER_OXCALC_iterative_cycle_config.md`.

## 7. Calc-State Display

The engine maintains an invalidation vocabulary (`clean` / `dirty_pending` / `needed` / `evaluating` / `verified_clean` / `publish_ready` / `rejected_pending_repair` / `cycle_blocked`). TreeCalc does not distinguish `verified_clean` from `clean` in user-visible state. The full displayed-status taxonomy is settled by UX design. Whether a cycle is an error or is iterated is profile-governed — see §7a.

## 7a. Circular References and Iterative Calculation

A reference cycle (a node that, directly or transitively, depends on itself) is a first-class, **profile-governed** condition — not merely an error. How a cycle is handled is set by **host-supplied configuration on the workspace**, exactly as Excel exposes circular-reference handling at the workbook level. The cycle semantics themselves are owned by the engine (OxCalc, `docs/spec/core-engine/w048-cycles/`); TreeCalc selects the profile, supplies the bounds, persists the configuration, and surfaces the result.

**Cycle profiles** (OxCalc W048; selected via the compatibility basis the host submits with a recalc):

| Profile | When | Behavior |
|---|---|---|
| `cycle.non_iterative_stage1` | **default** (≡ Excel with iterative calc off) | The cycle region is rejected: members enter `cycle_blocked`, the wave publishes no new cycle values, and previously published values remain visible (not republished). A circular-reference diagnostic is surfaced. |
| `cycle.excel_match_iterative` | opt-in; Excel-faithful | The region is iterated in workbook chain order (sequential region update), bounded by **Maximum Iterations** and a **Maximum Change** stop metric (default `0.001`, max absolute visible numeric delta). On convergence — or at the iteration bound, or at a stable oscillation terminal — the whole region publishes atomically, then dependents recompute. Matches Excel's iterative calculation for the covered surfaces. |
| `cycle.iterative_deterministic_v0` | opt-in; deterministic | A deterministic Jacobi-snapshot iteration: initial vector = last published numeric value (or zero), default **100** iterations / **`0.001`** threshold. Converged → publish the region atomically; divergent / oscillating / non-numeric → reject with no publication. Preferred where deterministic clarity matters more than reproducing Excel's history-sensitive behavior. |

**Host-supplied configuration (workspace-level, Excel-aligned).** The workspace persistence file records the cycle configuration alongside the capability profile (§4):

- **iterative calculation** enabled/disabled, and which iterative profile;
- **Maximum Iterations** (Excel `MaxIterations`);
- **Maximum Change** (the convergence threshold; Excel default `0.001`).

These mirror Excel's *File ▸ Options ▸ Formulas ▸ Enable iterative calculation / Maximum Iterations / Maximum Change*. The default is non-iterative (`cycle.non_iterative_stage1`), matching Excel out of the box. The selected profile and bounds are conveyed to OxCalc through the compatibility basis carried on the recalc snapshot; **TreeCalc does not implement iteration itself**.

**Calc-state and UX.** Under the non-iterative default, cycle members display `cycle_blocked` (§7) and surface in the workspace error summary and the "show cycle" affordance (`ux/REQUIREMENTS.md`). Under an iterative profile, members display their iterated values, and the engine's `cycle_iteration_trace` is available for diagnostics.

**Excel-alignment boundary.** The iteration model, convergence, and Excel-match values are Excel-anchored and owned by OxCalc (W048). TreeCalc owns the configuration surface, its persistence, and the calc-state / cycle-map UX. OxCalc currently has single-host-scoped evidence for the declared W048 surface, including the formerly open root/report-cell and non-numeric/blank/error prior-state lanes; this is still not a broad cross-version Excel-compatibility claim, and multithread behavior remains a profile/scope dimension rather than something TreeCalc should silently imply.

## 7b. Templates

Templates are host-level reusable subtree definitions instantiated at one or more positions in the workspace. They are a pure DNA TreeCalc affordance — OxCalc gains no template concept; the engine sees ordinary structural edits.

**Template storage and registry.** A template lives as a meta-flagged subtree in the workspace tree — a node with `is_meta = true` and the descendants that form the template's structural pattern. Templates can be placed at any tree position (workspace root, sheet level, deeper) — host convention typically groups them under a `Templates` container at some level. The template's nodes and their descendants are invisible to formulas; their formulas are stored as text but not bound or evaluated. See [`META_NODES.md`](META_NODES.md) for the meta-node model.

The host also keeps template bookkeeping: template id, template root node id, version, child/source-node ids, and instance counts/links. This registry is not a second copy of the template body; it is the host's index over the meta-subtree and its children so the UI can list templates, find rollouts, and validate mappings cheaply.

**Instances and rollout tags.** An instance is a subtree whose structure was generated from a template. The host tracks the link: instance root id/path, template id, bound template version, and a simple template-node-id to instance-node-id mapping. The rollout may also carry hidden meta-node children on the instance root and/or copied nodes with the same bookkeeping values (`template_id`, `template_version`, `template_node_id`). These tags are host data only; formulas cannot see them. They make it cheap to revalidate an instance, open the source template, or diff the current subtree against the template later.

**Instance editing and revalidation.** Post-rollout editing is deliberately simple. An instance is an ordinary subtree: users can edit formulas, names, children, and formats without the host maintaining detailed live template-state records. When the user asks to sync, validate, or compare, the host uses the saved template id mapping and hidden tags to line the instance up against the current template, compute a diff at that moment, and present/apply ordinary structural edits as needed. If the mapping is missing or too stale, the instance can be reported as detached or needing manual reconciliation. Do not build complex always-on template-specific edit tracking for v1.

**Sync.** Editing a template increments the template version. A sync request:
1. Loads the template subtree and the instance's template-node mapping.
2. Computes a current diff between template and instance.
3. Applies any accepted changes via ordinary OxCalc structural-edit operations.
4. Updates the instance's bound template version and bookkeeping tags.

When OxCalc gains transactional batch editing (see §6.8), each per-instance sync becomes one transaction; until then, sync is N individual edits with the host's undo log grouping them visually.

**Instantiation meta-copy rule.** Instantiation copies the template's regular structural pattern into a regular position: copied content nodes get `is_meta = false`. Template-internal meta children that represent host data on the copied nodes, such as `Format`, remain meta on the instance. The canonical examples in [`META_NODES.md`](META_NODES.md) guide this distinction.

**Parameters and cross-workspace templates** are not in scope. Walk-up scope handles most cross-subtree context needs (an instance formula can reach `^^.[Year]` to find its enclosing year by tree position). Cross-workspace templates would require external template version tracking which is more machinery than the value justifies right now.

**Operations:**

- **Promote to template** — convert an existing subtree into a template definition; replace its original location with an instance link.
- **Instantiate** — materialize template structure at a path; register the instance link. Bulk variant: instantiate at multiple paths in one user gesture.
- **Edit template** — modify the template definition and bump its version; instances sync when requested or when the UI offers an accepted sync action.
- **Edit instance** — ordinary structural/formula/format edits on the rollout; later validation/sync computes the diff from the template mapping.
- **Detach instance** — drop the instance link; subtree becomes independent.
- **Fit-check** (future) — given an arbitrary subtree, report which templates it could be mapped to and what current diff would result.

## 8. Structural Editing

User gestures on the tree:

- **Insert child / insert sibling** — creates a new node with default empty formula.
- **Create / rename** — enforce case-insensitive sibling-name uniqueness across regular and meta children. A parent cannot contain two children whose names differ only by case, even if one is meta.
- **Insert / reorder** — preserve the parent's stable sibling order. That order is persisted and is the order used by `@INDEX`, `@PREV`/`@NEXT`, `@PRECEDING`/`@FOLLOWING`, `.*`, and other ordered reference collections.
- **Rename** — prompts the user whether to propagate the rename to referencing formulas, showing the list of references that would be affected.
- **Move** — analogous prompt for any references that would resolve differently after the move (relative-path refs may rebind; absolute-path refs may break to `Unresolved`).
- **Delete** — references to the deleted node become `Unresolved`; undo affords recovery.

The engine-side semantics for each gesture (rebind vs. recalc vs. publication consequence) are the open piece in §6.5 — UI surfaces the engine's resolution rather than pretending the engine guarantees more.

## 9. Compact Grammar Sketch

```
Path           := Anchor? Segment SheetSep? ('.' Segment)* StructuredRefTail? // SheetSep = '!' allowed only at first separator position
                |  Anchor                                // anchor alone (root ref, parent ref, etc.)
SheetSep       := '!'                                    // separator alias, accepted only at position 1 (after first segment)
Anchor         := '[' AnchorContent? ']'                 // workspace selector or bracket-escaped first segment (see §3.3 ambiguity)
                |  '^' ('^')*                            // up-steps (ancestor)
AnchorContent  := Name                                   // workspace alias/path token OR escaped first node segment; bind disambiguates
                |  QuotedPath                            // quoted external workspace path
                |  '@' (SpecifierName | '[' Name ']')    // implicit-row (column-formula context only)
Segment        := Identifier
                |  BracketEscapedName                    // [Sales Q1], [$Forecast], etc.
                |  '*'                                   // children sugar (only as final segment)
                |  '**'                                  // recursive descent
StructuredRefTail := '[' StructuredRefContent ']'         // only when LHS is Table/data-type/structured-ref capable
Identifier     := [A-Za-z_][A-Za-z0-9_\\]*               // TreeCalc bare path identifier; deliberately excludes dot
BracketEscapedName := '[' Name ']'                       // any chars allowed; reserved chars escaped with '
Name           := (regular-char | "'" reserved-char | "''")+
                 where reserved-char ∈ { '[', ']', '#', "'", '@' }
StructuredRefContent := '#' SpecifierName | '@' Name | Name | composite-structured-ref
SpecifierName  := [A-Z][a-zA-Z0-9_]*
QuotedPath     := "'" (any-char-or-'')* "'"              // single-quoted path string for external workspaces

// Note: meta-flagged nodes (is_meta = true) have no formula-language syntax. They are
// addressable only through host-level operations, not through formulas. Walk-up scope
// and positional operators skip them.
```

Illustrative — the canonical grammar lives in OxFml's binding layer once TreeCalc-specific reference parsing is added.

The grammar above is the `=`-formula branch. An unprefixed entry is a literal constant resolved by the entry classification in §6, not by this grammar — a leading `=` selects the formula branch; a leading `'` forces text.

## 10. Excel Defined-Name Import

DNA TreeCalc imports Excel workbooks restricted to **sheets and defined names** (no cell references, no ranges, no Tables, no grid usage) with **zero formula source-text rewriting** in the common case. The grammar choices in §3.1–§3.4 are designed to make this possible.

### 10.1 Structural mapping

| Excel | TreeCalc |
|---|---|
| Workbook | Workspace |
| Sheet `Sheet1` | First-level node under root, named `Sheet1` |
| Sheet `Sales Q1` | `[Sales Q1]` under root (bracket-escape for space) |
| Workbook-scoped defined name `MyName` | `MyName` directly under root |
| Workbook-scoped defined name `My.Region.Sales` | Unrolled to path: root's child `My`, child `Region`, child `Sales` — three nodes, with the formula/value carried on `Sales` |
| Sheet-scoped defined name `Total` on Sheet1 | `Sheet1.Total` |
| Sheet-scoped defined name `Region.Total` on Sheet1 | Unrolled: `Sheet1.Region.Total` |

**Dots in Excel names are interpreted as path separators**, not as literal characters. The TreeCalc tree gains the hierarchy the dots suggested. A defined name `My.Region.Sales = 100` creates three nodes (`My`, `My.Region`, `My.Region.Sales`); the formula lives on the leaf.

Unrolling is incremental within the Excel name's scope. If a prefix already exists as a real defined name, it keeps its formula/value and is not replaced by a stub. Only missing prefixes become structural stubs. Excel's own uniqueness rules apply within each scope, so the importer treats workbook-scoped and sheet-scoped names as separate scope roots.

### 10.2 Intermediate stub nodes

When Excel has a dotted name whose prefixes are not themselves defined names, the unroll creates structural intermediates with no original Excel formula. The import policy gives these intermediates an explicit `#NAME?`-producing formula so that references to them match Excel's "no such name" behavior:

```
TreeCalc post-import:
  .My              value/formula result: #NAME? (explicit missing-name error)
  .My.Region       value/formula result: #NAME? (explicit missing-name error)
  .My.Region.Sales formula: <the Excel formula>
```

This matches Excel's `=My` → `#NAME?` when no defined name `My` exists. Do not use `NA()` for these stubs: `NA()` produces `#N/A`, which is a different Excel error. The exact representation can be a literal error value or a host-provided formula/error producer, but the observable result must be `#NAME?`. The user can later replace the stub with a real formula/value to make the intermediate node meaningful (TreeCalc allows non-leaf nodes to have formulas).

If Excel also has `My = 10`, then only the missing `My.Region` prefix is stubbed:

```
TreeCalc post-import:
  .My              formula: =10 (real imported defined name)
  .My.Region       value/formula result: #NAME? (explicit missing-name error)
  .My.Region.Sales formula: <the Excel formula>
```

### 10.3 Formula syntax compatibility

Excel formulas in defined-name-only workbooks import without source-text rewriting:

| Excel formula | Reads as in TreeCalc |
|---|---|
| `=MyName` | bare name; walk-up resolves to nearest `MyName` (sheet or workbook level — same scoping result Excel gives) |
| `=My.Region.Sales` | walk-up `My`, descend `.Region.Sales` |
| `=Sheet1!My.Region.Sales` | `Sheet1` + `!`-as-sheet-separator + descent → `.Sheet1.My.Region.Sales` |
| `=[Other.xlsx]Sheet2!Foo` | `[Other.xlsx]` is registered as a workspace alias at import; `!` after the alias separates; rest is path |
| `=SUM(A.X, A.Y, A.Z)` | unchanged |
| `=LET(x, My.X, x * 2)` | unchanged |
| `=LAMBDA(x, x * My.Rate)` | unchanged |
| `=INDIRECT("Sheet1!Foo")` | INDIRECT parses its string argument; `!`-after-first-segment is honored inside the string too, so the static-string case works without modification |

Scope semantics align: Excel's "sheet-scope wins, fall back to workbook-scope" maps onto TreeCalc's walk-up traversal of `caller's children → ancestor's children → root's children`. A workbook-scoped `Foo` imports as root child `Foo`; a Sheet1-scoped `Foo` imports as `Sheet1.Foo`. A formula evaluated under `Sheet1` resolves bare `Foo` to `Sheet1.Foo`; a formula evaluated under another sheet resolves bare `Foo` to that sheet's local `Foo` if present, otherwise the workbook-scoped root `Foo`.

### 10.4 Import preview and manifest

The importer emits a preview/manifest alongside the imported workspace. This is bookkeeping, not a second semantic model. For each Excel defined name and each created structural node, the manifest records:

- source workbook and sheet scope (`workbook` or the sheet name);
- original Excel name and formula source text;
- TreeCalc path and node id;
- whether the node was created as a stub;
- cross-workbook aliases that were registered or still need path confirmation;
- import warnings/errors such as grid-position functions, dynamic-grid `INDIRECT`, hidden internals, or name/scope collisions discovered in the source.

The preview uses this manifest to show the hierarchy that will be created, the `#NAME?` stubs, and any aliases or unsupported surfaces before the user accepts the import.

### 10.5 Cross-workbook reference handling

Excel cross-workbook syntax `[Other.xlsx]Sheet1!Foo` maps to TreeCalc's cross-workspace bracket `[Other.xlsx]Sheet1.Foo` (with `!`-after-sheet allowed). The importer:

1. Registers the workbook filename as a workspace alias in the workspace manifest.
2. Resolves the alias to a workspace file path (user may need to confirm the location).
3. Leaves the formula's `[Other.xlsx]` text intact — the workspace alias has that name.

### 10.6 What doesn't import cleanly

Beyond the no-grid constraint (which excludes A1-style refs, ranges, Tables, multi-cell array formulas), three categories require attention:

1. **Dynamic-string `INDIRECT`** — `=INDIRECT("Sheet1!" & A1)` builds the path at runtime. The string isn't subject to import rewriting. With the `!`-after-sheet allowance in INDIRECT's parsed string, static-string INDIRECT calls work, but dynamic strings that build cell refs (`"A" & N`) are grid-using and out of scope.

2. **Grid-position functions** — `ROW()`, `COLUMN()`, `CELL()`, `ADDRESS()`, `OFFSET()`, `SHEET()`, `SHEETS()`, `INFO()`. These reference grid positions and have no defined-name analog. If present in input formulas, they error at evaluation. They rarely appear in pure-defined-name workbooks.

3. **Hidden Excel internals** — VBA code, named styles, conditional-formatting rules tied to cells, drawing objects. Out of scope by the no-grid constraint.

### 10.7 Trade-off: unroll changes rename semantics

Excel treats `My.Region.Sales` as a single literal identifier; renaming a hypothetical `My` defined name doesn't propagate to `My.Region.Sales`. TreeCalc treats it as a path; renaming the `My` node propagates to all descendants (with the standard rename-propagation prompt).

For the typical case where dotted names were already meant hierarchically, this matches user intent. For pathological cases where dotted names were flat identifiers, renames behave differently than Excel. Import preserves the user's data; only the editability characteristics shift.

### 10.8 Bidirectional considerations

Saving a TreeCalc workspace back as Excel (the reverse direction) requires flattening:
- Each tree path becomes a workbook-scoped defined name with dots in the identifier (e.g., `My.Region.Sales`).
- Or sheet-scoped if the path starts with a sheet-equivalent.
- TreeCalc-novel surface (`@PREV`, `.*`, `**`, `@ANCESTORS`, ref-array literals, etc.) does NOT round-trip — those formulas error or lose semantics on save.
- Workspaces that stay within "named nodes with formulas" round-trip cleanly.

Bidirectional fidelity is a graduated promise: the further a workspace uses TreeCalc's novel operators, the more it diverges from what Excel can carry.

---
