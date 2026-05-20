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

- A **name** (case-insensitive at lookup, display preserves user casing).
- A **position** among its siblings (`sibling_index`, user-reorderable).
- A **formula** (OxFml source text) OR a **literal value** (typed in directly, can be a dynamic array).
- A computed **value** (`EvalValue` — scalar, array, error, reference, lambda).
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
- Bare name — sibling lookup (no anchor). Walk-up resolution skips meta-flagged nodes and their descendants; they are not reachable from formulas.

### 3.2 Path forms

```
Margin                            sibling-only lookup of "Margin" among caller's siblings
Q1.Margin                         sibling "Q1", then its child "Margin"
^                                 caller's parent (single reference)
^.Margin                          parent's child "Margin"
^^.Total                          grandparent's child "Total"
^^^                               great-grandparent
[]                                the workspace root (single reference)
[]Sheet1.Margin                   workspace-rooted absolute path
[].@CHILDREN                      workspace root's children (all top-level "sheets")
[Sales Q1].Margin                 bracket-escaped segment (for names with spaces / punctuation)
[ws]Branch1.MyNode                cross-workspace reference (see §3.3)
ref.@NAME                         meta-accessor: node's display name (see §3.5)
ref.*                             children-as-references collection (sugar for ref.@CHILDREN)
@PREV.Net                         previous sibling's Net
@NEXT.Margin                      next sibling's Margin
```

**Bare names resolve by lexical walk-up.** Search caller's own children first, then each ancestor's children one scope at a time (parent's children, grandparent's children, ..., up to root's children). First match wins. If no match in any scope, the reference is `Unresolved`. This generalizes Excel's nearest-scope-wins rule (sheet scope, then workbook scope) from two levels to arbitrary tree depth.

Consequences:
- An ancestor is findable by its own name (`Q1` from inside `Q1.Income.Sales` resolves to `Q1` as a child of `2005`).
- Self-reference (a node's formula writing its own name) resolves to self and produces the standard cycle error.

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
| Start of expression | `[]` | workspace root |
| Start of expression | `[#Name]` | self-anchored meta-specifier |
| Start of expression | `[@Name]` | implicit current-row column (column-formula context only) |
| Start of expression | `['quoted path']` | workspace selector (always) |
| Start of expression | `[Word]` / `[Word with spaces]` | workspace selector if the content matches a registered alias; otherwise bare-escaped node name resolved via walk-up |
| After a path, no separator | `path[Col]` | table column ref (LHS Table-typed) |
| After a path, no separator | `path[#Spec]` | meta-specifier on path |
| After a path, no separator | `path[@Col]` | implicit current-row column (column-formula context) |
| After a `.` separator | `path.[Name]` | descent to a node whose name needs escaping |
| Double-bracket form | `path[[Sub]:[Sub2]]`, `path[[#Headers],[Col]]` | structured-ref composite per Excel rules |

The start-of-expression `[Word]` ambiguity (workspace alias vs. bare-escaped name) is resolved at bind time: the manifest is consulted first, and a bare-escaped name lookup is the fallback. Real conflicts (user registers an alias whose name collides with a workspace-global node) are rare; the editor surfaces the resolved binding on hover.

Engine mapping: bind layer produces an `External(ExternalRef)` carrier (OxFml). From OxCalc-Tree's view, this is a `HostSensitive` reference. The host adapter loads external workspaces, caches their published values, and signals invalidation on external republish.

### 3.4 Node names and bracket escaping

Following Excel's structured-reference rules for column names (Microsoft's [structured-references doc](https://support.microsoft.com/en-au/office/using-structured-references-with-excel-tables-f5ed2452-2337-4f71-bed3-c8ae6d2b276e)):

- **Bare identifier** (no escape required): `[A-Za-z_][A-Za-z0-9_\\]*` — letters, digits (not leading), underscore, backslash. Matches Excel's defined-name character set (Excel allows `\` in names; we follow). Dots are NOT in the bare identifier set — `.` is always the separator.
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
| `@INDEX` | `@INDEX` | `ref.@INDEX` | number (sibling position) |
| `@PARENT` | (`^` is the shorthand) | `ref.@PARENT` | single ref |
| `@CHILDREN` | (`.*` is the shorthand on this-node, but rare) | `ref.@CHILDREN` (= `ref.*`) | set of refs |
| `@PREV` | `@PREV` | `ref.@PREV` | single ref (errors on out-of-range) |
| `@NEXT` | `@NEXT` | `ref.@NEXT` | single ref (errors on out-of-range) |
| `@PRECEDING` | `@PRECEDING` | `ref.@PRECEDING` | set of refs (earlier siblings, sibling-index order) |
| `@FOLLOWING` | `@FOLLOWING` | `ref.@FOLLOWING` | set of refs (later siblings, sibling-index order) |
| `@ANCESTORS` | `@ANCESTORS` | `ref.@ANCESTORS` | set of refs (closest first, up to root) |

Free-standing forms are sugar for `THIS.@FOO` where `THIS` is the implicit calling node. TreeCalc does not expose a `THIS` token; the free-standing form is always available where applicable.

### 3.5b Navigation principle

Path-navigation operators are either **deterministic (single result)** or **set-producing (collection)**. No operator implicitly searches multiple candidate positions and picks one. Closest-match / first-match selection is always explicit composition via `FILTER` + `INDEX`.

| Operator | Single or set? |
|---|---|
| Bare name, descent (`Foo.Bar`), workspace anchor (`[]Foo`, `[ws]Foo`) | single |
| Parent (`^`), fixed-depth ancestor (`^^`, `^^^`, ...), workspace root (`[]` alone) | single |
| `@PREV`, `@NEXT`, `@PARENT` | single |
| `@NAME`, `@INDEX`, `@FORMULA` | single (value, not ref) |
| `.*` / `@CHILDREN`, `@ANCESTORS`, `@PRECEDING`, `@FOLLOWING`, `**` | set |

Example — "closest enclosing Year" is explicit composition, not a path-level operator:

```
INDEX(FILTER(@ANCESTORS, a -> a.@NAME = "Year"), 1)
```

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
| `[]` alone | reference to workspace root node |
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

Most design questions have a fast answer: what does Excel do? Capitalization (`@NAME` not `@name`), single-quoting (`'Sales Q1'` not backtick), case-insensitive name lookup, error-code shapes (`#REF!`, `#VALUE!`, `#CALC!`) all follow from this.

## 6. Values, Arrays, and Engine Prerequisites

**Per-node value type is `EvalValue`** — scalar `Number`/`Text`/`Logical`/`Error`, plus `Array` (a 2D grid of scalar `ArrayCellValue` cells), `Reference(ReferenceLike)`, `Lambda`.

**Dynamic arrays are first-class at every layer.** A formula result can be an array; a user-typed literal value can be an array (resized as edited); references to an array-valued node return the whole array.

**Nested arrays are rejected.** Operations that would produce an `EvalValue::Array` whose cells are themselves arrays — for example `MAP(Sheet1.*, c -> c.Margin)` when any Margin's value is itself an array — error out at bind or runtime with `#CALC!`.

**Reference collections are References, not Arrays.** A tree-reference collection (`.*`, `@CHILDREN`, `@ANCESTORS`, `@PRECEDING`, `@FOLLOWING`, `Foo.**`, and explicit `{Foo, Bar}` literals) is carried in `EvalValue::Reference(ReferenceLike)` — the same value-shape Excel uses for range references like `A1:B5`. The existing `ReferenceLike { kind: ReferenceKind, target: String }` extends to recognize tree-shaped kinds. Functions that already take a `Reference` input handle these via existing iteration code; no per-function arity extension is needed.

The following pieces are TreeCalc-specific extensions that depend on engine work in OxCalc / OxFml / OxFunc:

1. **New `ReferenceKind` variants** in OxFunc: tree single-node, tree node-set (children/ancestors/preceding/following/descendants/explicit), tree-dynamic (runtime-resolved via INDIRECT). Iteration logic for the new kinds plugs into the existing per-kind dispatch.
2. **`SelfNode` base variant** in OxCalc's `RelativePath` (or `Ancestor(0)` generalization) so the walk-up scope resolution can capture own-child matches.
3. **Set-membership dependency edge type** in OxCalc's `DependencyGraph`: "this formula depends on the set of children of node X" must invalidate when the set's membership changes structurally (add / remove / rename of a matching child). Same machinery covers `@ANCESTORS`, `@PRECEDING`/`@FOLLOWING`, and `**`.
4. **Reference-array literals** `{Foo, Bar}` — OxFml binder grammar admits references inside `{...}` (not only scalars) under the `treecalc-v1` profile.
5. **Cross-workspace orchestration** via `HostSensitive` — the host adapter loads external workspaces, caches their published values, and signals invalidation on external republish.
6. **Structural-edit semantics** — node rename / move / formula replacement / add / remove: when does each force rebind vs. recalc vs. publication-visible structural delta. Specified in `OxCalc/docs/spec/core-engine/CORE_ENGINE_TREECALC_SEMANTIC_COMPLETION_PLAN.md`. The rename-propagation prompt UX depends on the engine's resolution.
7. **Profile-aware `INDIRECT`** — string argument parsed as a tree path under `treecalc-v1`, as A1/R1C1 under `strict-excel`. Function lives in OxFunc; parsing dispatches on the active profile.
8. **Transactional batch structural editing** in OxCalc — `begin / N edits / commit-or-rollback` atomically as one publication candidate. Useful broadly (multi-node refactors, paste, undo) and specifically needed for clean template sync. Queued as a fundamental engine feature.
9. **`is_meta` per-node attribute** (boolean, default false). When true on any node, that node and its entire descendant subtree are invisible to formula reference resolution AND are not bound or evaluated by the engine. Bind layer skips meta-flagged nodes and their descendants when resolving references (resulting in `Unresolved` on lookup failure). Positional operators (`@PREV`/`@NEXT`/`@INDEX`/`.*`) on regular siblings skip meta neighbors. Templates and per-node format both use this flag; future meta-node uses (configuration, named lambdas, annotations) inherit the same mechanism. Storage / filter strategy is OxCalc's choice.
10. **Conditional-formatting rule semantics for the format surface** — confirm or add support for ordered multiple rules, `Stop If True`, action accumulation across rules, and subtree-level format inheritance inputs. The format engine details live in OxFml/OxFunc, but TreeCalc's format editor and Excel verification depend on this behavior being explicit and reusable rather than reconstructed in the host.

## 7. Calc-State Display

The engine maintains an invalidation vocabulary (`clean` / `dirty_pending` / `needed` / `evaluating` / `verified_clean` / `publish_ready` / `rejected_pending_repair` / `cycle_blocked`). TreeCalc does not distinguish `verified_clean` from `clean` in user-visible state. The full displayed-status taxonomy is settled by UX design.

## 7b. Templates

Templates are host-level reusable subtree definitions instantiated at one or more positions in the workspace. They are a pure DNA TreeCalc affordance — OxCalc gains no template concept; the engine sees ordinary structural edits.

**Template storage.** A template lives as a meta-flagged subtree in the workspace tree — a node with `is_meta = true` and the descendants that form the template's structural pattern. Templates can be placed at any tree position (workspace root, sheet level, deeper) — host convention typically groups them under a `Templates` container at some level. The template's nodes and their descendants are invisible to formulas; their formulas are stored as text but not bound or evaluated. See [`META_NODES.md`](META_NODES.md) for the meta-node model.

**Instances.** An instance is a subtree whose structure was generated from a template. The host tracks the link: instance root path, template id, bound template version, and the divergence record.

**Divergence tracking.** Per-leaf override is the granularity. When a user edits an instance leaf's formula, the host records that node-path as diverged from the template. Future template-source edits to that leaf are skipped for this instance. Instances may also acquire structural divergence — added children, removed children, modified inner structure — and these are recorded explicitly in the instance metadata. The divergence data structure is shaped to support a future "fit-check" operation: given an arbitrary subtree, does it match a template's shape closely enough to be retroactively adopted as an instance?

**Sync.** Editing a template:
1. Host computes the structural diff between previous and new template versions.
2. For each instance whose bound version was the previous version: host applies the diff via ordinary OxCalc structural-edit operations, skipping paths that match the instance's divergence record.
3. Instance bound-version advances to the new template version.

When OxCalc gains transactional batch editing (see §6.8), each per-instance sync becomes one transaction; until then, sync is N individual edits with the host's undo log grouping them visually.

**Parameters and cross-workspace templates** are not in scope. Walk-up scope handles most cross-subtree context needs (an instance formula can reach `^^.[Year]` to find its enclosing year by tree position). Cross-workspace templates would require external template version tracking which is more machinery than the value justifies right now.

**Operations:**

- **Promote to template** — convert an existing subtree into a template definition; replace its original location with an instance link.
- **Instantiate** — materialize template structure at a path; register the instance link. Bulk variant: instantiate at multiple paths in one user gesture.
- **Edit template** — modify the template definition; trigger sync to all instances.
- **Edit instance leaf** — record divergence on that leaf; apply the edit normally.
- **Detach instance** — drop the instance link; subtree becomes independent.
- **Fit-check** (future) — given an arbitrary subtree, report which templates it could be adopted as an instance of and what divergences would be recorded.

## 8. Structural Editing

User gestures on the tree:

- **Insert child / insert sibling** — creates a new node with default empty formula.
- **Rename** — prompts the user whether to propagate the rename to referencing formulas, showing the list of references that would be affected.
- **Move** — analogous prompt for any references that would resolve differently after the move (relative-path refs may rebind; absolute-path refs may break to `Unresolved`).
- **Delete** — references to the deleted node become `Unresolved`; undo affords recovery.

The engine-side semantics for each gesture (rebind vs. recalc vs. publication consequence) are the open piece in §6.5 — UI surfaces the engine's resolution rather than pretending the engine guarantees more.

## 9. Compact Grammar Sketch

```
Path           := Anchor? Segment SheetSep? ('.' Segment)*  // SheetSep = '!' allowed only at first separator position
                |  Anchor                                // anchor alone (root ref, parent ref, etc.)
SheetSep       := '!'                                    // separator alias, accepted only at position 1 (after first segment)
Anchor         := '[' AnchorContent? ']'                 // workspace selector (or self-meta when content starts with #/@)
                |  '^' ('^')*                            // up-steps (ancestor)
AnchorContent  := Identifier                             // workspace alias OR bare-escaped node name
                |  QuotedPath                            // quoted external workspace path
                |  '#' SpecifierName                     // self-anchored meta-specifier (e.g. [#Prev], [#Name])
                |  '@' (SpecifierName | '[' Name ']')    // implicit-row (column-formula context only)
Segment        := Identifier
                |  BracketEscapedName                    // [Sales Q1], [$Forecast], etc.
                |  MetaSpecifier                         // [#Name], [#Children], [#Prev], ...
                |  '*'                                   // children sugar (only as final segment)
                |  '**'                                  // recursive descent
Identifier     := [A-Za-z_][A-Za-z0-9_\\]*               // letters, digits, underscore, backslash; matches Excel defined-name char set
BracketEscapedName := '[' Name ']'                       // any chars allowed; reserved chars escaped with '
Name           := (regular-char | "'" reserved-char | "''")+
                 where reserved-char ∈ { '[', ']', '#', "'", '@' }
MetaSpecifier  := '[' '#' SpecifierName ']'              // Title-Case per Excel convention (e.g. [#Prev], [#Name])
SpecifierName  := [A-Z][a-zA-Z0-9_]*
QuotedPath     := "'" (any-char-or-'')* "'"              // single-quoted path string for external workspaces

// Note: meta-flagged nodes (is_meta = true) have no formula-language syntax. They are
// addressable only through host-level operations, not through formulas. Walk-up scope
// and positional operators skip them.
```

Illustrative — the canonical grammar lives in OxFml's binding layer once TreeCalc-specific reference parsing is added.

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

### 10.2 Intermediate stub nodes

When Excel had `My.Region.Sales` but no `My.Region` and no `My` defined names, the unroll creates structural intermediates with no original Excel formula. The import policy gives these intermediates an explicit `#NAME?`-producing formula so that references to them match Excel's "no such name" behavior:

```
TreeCalc post-import:
  .My              formula: =NA() or equivalent #NAME?-producer
  .My.Region       formula: =NA() or equivalent
  .My.Region.Sales formula: <the Excel formula>
```

This matches Excel's `=My` → `#NAME?` when no defined name `My` exists. The user can later replace the stub formula with a real one to make the intermediate node meaningful (TreeCalc allows non-leaf nodes to have formulas).

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

Scope semantics align: Excel's "sheet-scope wins, fall back to workbook-scope" maps onto TreeCalc's walk-up traversal of `caller's children → ancestor's children → root's children`. A defined name `Foo` workbook-scoped and `Foo` sheet-scoped on Sheet1 resolves correctly from each context.

### 10.4 Cross-workbook reference handling

Excel cross-workbook syntax `[Other.xlsx]Sheet1!Foo` maps to TreeCalc's cross-workspace bracket `[Other.xlsx]Sheet1.Foo` (with `!`-after-sheet allowed). The importer:

1. Registers the workbook filename as a workspace alias in the workspace manifest.
2. Resolves the alias to a workspace file path (user may need to confirm the location).
3. Leaves the formula's `[Other.xlsx]` text intact — the workspace alias has that name.

### 10.5 What doesn't import cleanly

Beyond the no-grid constraint (which excludes A1-style refs, ranges, Tables, multi-cell array formulas), three categories require attention:

1. **Dynamic-string `INDIRECT`** — `=INDIRECT("Sheet1!" & A1)` builds the path at runtime. The string isn't subject to import rewriting. With the `!`-after-sheet allowance in INDIRECT's parsed string, static-string INDIRECT calls work, but dynamic strings that build cell refs (`"A" & N`) are grid-using and out of scope.

2. **Grid-position functions** — `ROW()`, `COLUMN()`, `CELL()`, `ADDRESS()`, `OFFSET()`, `SHEET()`, `SHEETS()`, `INFO()`. These reference grid positions and have no defined-name analog. If present in input formulas, they error at evaluation. They rarely appear in pure-defined-name workbooks.

3. **Hidden Excel internals** — VBA code, named styles, conditional-formatting rules tied to cells, drawing objects. Out of scope by the no-grid constraint.

### 10.6 Trade-off: unroll changes rename semantics

Excel treats `My.Region.Sales` as a single literal identifier; renaming a hypothetical `My` defined name doesn't propagate to `My.Region.Sales`. TreeCalc treats it as a path; renaming the `My` node propagates to all descendants (with the standard rename-propagation prompt).

For the typical case where dotted names were already meant hierarchically, this matches user intent. For pathological cases where dotted names were flat identifiers, renames behave differently than Excel. Import preserves the user's data; only the editability characteristics shift.

### 10.7 Bidirectional considerations

Saving a TreeCalc workspace back as Excel (the reverse direction) requires flattening:
- Each tree path becomes a workbook-scoped defined name with dots in the identifier (e.g., `My.Region.Sales`).
- Or sheet-scoped if the path starts with a sheet-equivalent.
- TreeCalc-novel surface (`@PREV`, `.*`, `**`, `@ANCESTORS`, ref-array literals, etc.) does NOT round-trip — those formulas error or lose semantics on save.
- Workspaces that stay within "named nodes with formulas" round-trip cleanly.

Bidirectional fidelity is a graduated promise: the further a workspace uses TreeCalc's novel operators, the more it diverges from what Excel can carry.

---
