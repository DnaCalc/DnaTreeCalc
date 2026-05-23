# DNA TreeCalc — Test Corpus

The durable, hand-authored **test corpus** for DNA TreeCalc: spec-derived cases that pin the
behavior of TreeCalc's *novel* surface. It follows the DNA Calc family convention of keeping a
curated corpus under `docs/test-corpus/` (cf. `OxCalc`, `OxReplay`, `OxXlPlay`).

The corpus is **declarative and language-neutral** — canonical JSON with stable field names, per
the family scenario-schema doc (`OxCalc/docs/spec/core-engine/CORE_ENGINE_TEST_SCENARIO_SCHEMA_AND_TRACECALC.md`).
It is authored *now*, before the Rust/WASM workspace exists, and is consumed two ways:

- **today** — a pwsh well-formedness validator (`tools/validate-corpus.ps1`); the local-tier check while there is no engine.
- **once W002 lands** — an executable runner that binds each case through the `OxCalcTree` bridge and asserts the expected outcome (see [Runner contract](#runner-contract)).

---

## How we test (gate tiers — `OPERATIONS.md` §6)

| Tier | What it covers here | Where it runs |
|---|---|---|
| **Local** | This corpus (spec-defined cases for the novel surface) + `cargo` unit/integration tests once code exists | this repo |
| **Excel anchor** | Whole-workspace value-equivalence — does a workspace recompute like Excel? | fixtures here (`workspaces/`) → replay bundle → **OxXlPlay** (construct + observe) + **OxReplay** (diff + govern). Excel comparison is never reimplemented here. |
| **Formal** | Reference / scope / set-operator semantics where load-bearing | candidate Lean/TLA+; keep semantics formally-traceable |

---

## What is a TreeCalc test vs. an OxCalc test? (the boundary)

The dividing line is the **Excel-alignment boundary** (`../model/CORE_MODEL_SPEC.md` §5) and the
**layering table** (§1). OPERATIONS §6 states it directly: the novel tree/reference/skin surface
"has no Excel counterpart and is pinned by spec-defined cases"; the Excel-aligned surface is
Excel-anchored and "never reimplemented here."

**Belongs here** (novel tree/reference surface):

- Path-surface syntax and its mapping to engine `TreeReference` variants: `.` separator, the `!`
  sheet-alias rule, `^`/`^^` ancestors, `[]`/`[ws]` selectors, bracket-escaping, `@`-meta-accessors,
  `.*`/`@CHILDREN`, `**`, `{…}` ref-array literals.
- **Walk-up resolution outcomes** — which node a reference binds to (or `Unresolved`) for a given tree + caller.
- Operator **cardinality** (single vs. set — §3.5b).
- **Capability-profile gating** — `treecalc-v1` accepts / `strict-excel` rejects (§4).
- **Meta-node invisibility** to resolution and positional operators (§2, §6 item 9).
- **Import mapping** — Excel defined-names → tree + stub policy (§10); export round-trip rules (§10.8).
- **Whole-workspace value-equivalence vs. Excel** — fixtures here; *executed* via the Excel-anchor tier, not by local re-evaluation.

**Belongs in OxCalc / OxFml / OxFunc** (useful, but NOT in this repo):

- Function/operator semantics, value coercion, error algebra, array lifting (OxFunc).
- Single-formula grammar/parse/bind/eval, format-code parsing, LAMBDA/LET (OxFml).
- Dependency graph, invalidation closure, epochs, atomic publication, and the engine's
  reference-variant *resolution machinery* (OxCalc — see its `TraceCalc` corpus).
- Number formats, dates, error-code shapes — Excel-aligned (OxFunc/OxFml), Excel-anchored via OxReplay.
- If TreeCalc work needs new engine behavior, raise a **handover** (`OPERATIONS.md` §7), not a local test.

**The grey line — walk-up resolution.** The engine's bind layer *computes* walk-up, but TreeCalc
*specifies* it (§3.2, §3.7) and owns the corpus of expected
`(tree, caller, reference) → target | Unresolved | reject` outcomes. These are the host's contract
on the engine: they run as acceptance checks through the bridge once it exists. If the engine
resolves differently than a case says, that is an engine bug → **handover**, not a corpus edit.

> For context, OxCalc's own corpus (`OxCalc/docs/test-corpus/core-engine/tracecalc/`) uses the
> abstract `TraceCalc` op vocabulary (`const`/`sum`/`choose`/`cap_gate`/…) to exercise
> candidate/publish/reject and invalidation. It deliberately never touches the reference surface.
> The two corpora are complementary: OxCalc pins the coordinator mechanics; TreeCalc pins the
> user-facing surface that maps onto them.

---

## Layout

```
docs/test-corpus/
  README.md            this file
  schema/              JSON Schemas for the case + workspace shapes
  workspaces/          shared multi-node tree fixtures (referenced by id)
  references/          §3 — walkup, anchors, sibling-offsets, classification, narrow active raw-children slice, set-membership, literals, node-functions, escaping, cross-workspace, meta, syntax
  profiles/            §4 — treecalc-v1 vs strict-excel gating
  constants/           §2/§6 — entry classification (constant vs formula)
  structural-edits/    §8 — rename/move/delete/insert propagation
  arrays/              §6 — dynamic array references
  dynamic-references/  §6 (item 1), §10.3 — INDIRECT-driven dynamic (CTRO) references
  cycles/              §7a — circular references under each cycle profile
  templates/           §7b — template instantiate/sync/divergence (META_NODES)
  formatting/          §6 (item 10) — Format meta-children + inheritance (META_NODES)
  tables/              §7c — table-node structured refs + column formulas (engine-unpacked)
  import/              §10 — Excel defined-name → tree mapping
  value-equivalence/   Excel anchor — whole-workspace recompute + export round-trip
  perf/                stress workloads (deep/wide/large-array/edit-storm) — timed runs, not value assertions (ux/TECHNICAL §7.6)
  tools/
    validate-corpus.ps1
```

Section references (`§3.2`, etc.) are into [`../model/CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) unless stated otherwise.

---

## Case conventions

- **Canonical JSON**, UTF-8, stable field names. One case file per theme; each file is
  `{ schema_version, theme, description, cases: [...] }` with `schema_version` = `treecalc-corpus-v1`.
- **Shared workspaces.** Tree fixtures live in `workspaces/<id>.json` as
  `{ schema_version, workspace_id, profile, nodes: [{ node_id, formula, is_meta? }] }`, where
  `node_id` is the node's **dotted path** (the identity surface) and sibling order is the array order.
  A node's `formula` is its single content field — `""` is the node-level `Empty` value, a leading
  `=` is a formula, and any other entry is a literal constant (Excel cell-entry rules, §6). Formula
  evaluation cannot produce top-level `Empty`; `=""` is an empty string text value. Resolution cases
  reference a
  workspace by id and name a `caller` node + a `reference` string.
- **Every case** carries a stable `id`, a human `name`, a `spec` citation (e.g. `"§3.2, §3.7"`),
  a `kind`, and asserts exactly one outcome.
- `expect.engine_ref` records the **intended** engine `TreeReference` mapping (§3.7) — informational
  today, asserted by the runner later.

### Case kinds

| `kind` | Required fields | `expect` shape |
|---|---|---|
| `resolution` | `workspace`, `caller`, `reference` | `{ outcome: resolved\|unresolved\|reject\|error, target?, engine_ref?, reason?, calc? }` (`target` is a `node_id`, or `/` for the workspace root) |
| `classification` | `reference` | `{ cardinality: single\|set\|value, result_kind? }` (§3.5b / §3.5) |
| `profile` | `reference`, `profiles` | `profiles` maps a profile id to `"accept"`/`"reject"` or `{ verdict, as? }` (§4) |
| `syntax` | `reference` | `{ parse: accept\|reject, equivalent_to?, reason? }` (§3.1, §3.3) |
| `import` | `excel` | `{ nodes: [{ node_id, formula, stub? }], aliases? }` **or** `{ outcome: out-of-scope\|eval-error, reason }` (§10) |
| `cycle` | `workspace`, `members`, `config` | `{ outcome: cycle_blocked\|published\|rejected, terminal?, publication?, values_anchor? }` — host-surfaced outcome per cycle profile (§7a); iterated *values* are Excel/engine-anchored, not asserted here |
| `dynamic` | `workspace`, `caller`, `reference` | `{ outcome: resolved\|unresolved\|error\|cycle_blocked, target?, depends_on?, engine_ref? }` + optional `given` (runtime selector values) — INDIRECT / CTRO dynamic references (§6 item 1, §10.3) |
| `membership` | `workspace`, `caller`, `reference` | `{ outcome: resolved\|unresolved, members: [], ordered?, engine_ref? }` — ordered reference collections such as `.*`, `@ANCESTORS`, `**`, and `{Foo, Bar}` (§3.5b, §6) |
| `edit` | `workspace`, `edit`, `caller?`, `reference?` | `{ outcome: resolved\|unresolved\|rebound\|error, rewritten_to?, rebinds_to? }` — post-edit binding consequence (§8) |
| `table` | `workspace`, `table`, `reference` | `{ outcome: resolved\|error, target?, target_kind?, engine_ref?, values_anchor? }` — table structured-reference lowering and column-formula binding (§7c) |
| `constant`, `template`, `format`, `value_equivalence` | kind-specific fields | Host/corpus fixtures outside the reference surface; see their theme descriptions. |

---

## Traceability & progressive activation

Every theme file declares the **workset** that owns it and an **activation status**, so the corpus
maps cleanly onto the work plan and can be switched on area-by-area as the engine/features land.

- `workset` — the [`WORKSET_REGISTER.md`](../WORKSET_REGISTER.md) id whose completion makes these
  cases runnable (e.g. `W004`).
- `status` — `pending` (authored + well-formed, but no executable runner yet) or `active` (wired to a
  runner/check). A theme flips to `active` only when a repo-local runner binds it through the real
  OxCalc bridge and asserts the declared outcome. Do not activate a theme with a TreeCalc-local
  parser or resolver.

`validate-corpus.ps1` prints a coverage matrix grouped by workset + status. The future runner selects
`status: active` themes; this validator checks the well-formedness of every theme regardless.

| Workset | Themes | What it pins |
|---|---|---|
| **W002** engine seam | `constants/`, `cycles/` | entry classification (§2/§6); cycle profiles (§7a) |
| **W004** reference model | `references/*`, `profiles/`, `dynamic-references/`, `structural-edits/`, `arrays/`, `tables/` | §3 resolution + set membership, §4 gating, INDIRECT/CTRO, §8 edit propagation, §6 arrays/reference literals, §3.8 node-as-function, §7c structured refs |
| **W007** meta/format/templates | `templates/`, `formatting/` | §7b templates; `Format` inheritance (META_NODES) |
| **W008** import | `import/`, `value-equivalence/import-*` | §10 defined-name import + recompute-equals-Excel |
| **W009** export/replay | `value-equivalence/export-*` | export round-trip; whole-workspace value-equivalence |
| **Tables** (W004, cross-repo prereq) | `tables/` | §7c structured-ref resolution + column formulas; activation waits for the table-node unpacking handover and then runs through the OxCalc bridge |
| cross-cutting | `perf/` | timed stress workloads (`ux/TECHNICAL.md` §7.6) — measured, not asserted pass/fail |

The matrix in the validator output is the live source of truth; this table is the human overview.

### Pending-to-active routes

Every family below starts as `pending`: the JSON is the durable contract, and
`validate-corpus.ps1` proves only well-formedness. A family becomes `active`
when the named runner path exists and passes through the OxCalc bridge. The
bridge runner owns parse/bind/resolve assertions; TreeCalc does not implement a
parallel parser to make activation possible.

| Corpus family | First active route | Activation gate |
|---|---|---|
| `references/walkup` | W005 corpus runner v1 | Bare walk-up and dotted descent bind through OxCalc; expected target/canonical path assertions pass. |
| `references/anchors`, `references/sibling-offsets` | W004 reference runner | Ancestor/root/workspace anchors and `@PREV`/`@NEXT` bind through OxCalc bridge variants. |
| `references/syntax`, `references/escaping`, `profiles/gating` | W004 parser/binder runner | OxFml parses under the OxCalc-supplied TreeCalc capability profile and rejects under `strict-excel` where specified. |
| `references/cross-workspace` | W004 external workspace runner | Host loads the aliased workspace, OxCalc receives the external/host-sensitive edge, and unavailable externals surface as diagnostics. |
| `references/meta-nodes`, `formatting/`, `templates/` | W004/W007 bridge runners | `is_meta` filtering is honored by binding/positional operators; W007 then consumes the same flag for format/template host data. |
| `references/children-raw-active` | W005 active children runner | The current public OxCalc prebind accepts free-standing raw `=SUM(@CHILDREN)` / `=SUM(.*)` and focused qualified raw `=SUM(base.@CHILDREN)` / `=SUM(base.*)` through OxCalc's public qualified-base query packet. The runner executes the workspace through `LiveOxCalcTreeBridge` and asserts published value plus dependency membership. |
| `references/ordered-raw-active` | W004 active ordered-selector runner | The current public OxCalc ordered-selector query/prebind accepts raw `@PRECEDING`, `@FOLLOWING`, `@ANCESTORS`, and recursive `Base.**.Margin` forms when DnaTreeCalc supplies only resolved base/member packets. The runner executes the workspace through `LiveOxCalcTreeBridge` and asserts published value plus dependency membership. |
| `references/set-membership`, `references/literals` | W004 reference-collection runner | Ordered reference collections (`@CHILDREN`/`.*`, `@PRECEDING`, `@FOLLOWING`, `@ANCESTORS`), recursive `**`, explicit reference literals, duplicate preservation, and mixed scalar/reference rejection are asserted through bridge results. The broad set-membership theme remains pending until the full family, including non-active base forms and literals, has public OxCalc bridge support. |
| `dynamic-references/indirect` | W004 CTRO runner | `INDIRECT` strings rebind at calc time through the engine overlay; TreeCalc asserts outcome/diagnostics only. |
| `references/node-functions` | W004 node-call runner | Single-reference lambda-valued nodes are invocable through OxFml/OxCalc; set-valued callees reject. |
| `tables/structured-references` | W004 active table runner | Structured refs lower through DnaTreeCalc's table fixture projection and OxCalc's public W056 table runtime path; TreeCalc asserts target kind, selected columns, `#All`, row-context values, bracket-escaped table/column cases, formula/totals results, dependency lowering, update impact, and bridge-retained table context identity. |
| `arrays/` | W004 array/reference runner | Array-valued nodes and reference collections travel through the bridge without inter-node spill; Excel-aligned scalar array values remain engine/Excel anchored. |
| `structural-edits/` | W004 structural-edit runner | Rename/move/delete/insert operations go through OxCalc structural edit APIs and post-edit binding consequences match cases. |
| `constants/`, `cycles/` | W002 bridge runners | Pending until the upstream channels are executable in the local bridge: constants wait for the OxFml TreeCalc entry-classification API, and cycle corpus activation waits for the OxCalc consumer facade to expose the typed `cycle_config` / `cycle_diagnostics` fields. The W002 bridge smoke itself is covered by the `LiveOxCalcTreeBridge` Rust tests. |
| `import/`, `value-equivalence/` | W008/W009 import/replay runners | Import/export flows emit replay bundles; OxXlPlay observes Excel and OxReplay governs comparison. |

### Current bridge activation floor

The W002 local Rust bridge smoke is implemented in
`src/dnatreecalc-host/src/adapters/oxcalc/live_bridge.rs`: it submits a tiny
named-node workspace through `LiveOxCalcTreeBridge`, receives published values,
dependency edges, node state, evaluation order, and OxCalc diagnostics, and is
run by `cargo test --workspace`. The authored W002 corpus themes remain
`pending`: `constants/entry-classification` is blocked on the OxFml TreeCalc
entry-classification response, while `cycles/cycle-profiles` is blocked on the
current OxCalc consumer facade not yet carrying the typed cycle request/result
fields. `validate-corpus.ps1` is therefore expected to report zero active W002
cases until those two upstream surfaces are executable.

As of the W004/W005 raw-prebind adoption, the local Rust bridge can submit
original TreeCalc formula text for the first `ChildrenV1` slice through OxCalc's
public `prebind_treecalc_formula_text(owner_node_id, source_text)` interface and
can consume OxCalc's public qualified-children base-query packet for the focused
`base.@CHILDREN` / `base.*` forms. DnaTreeCalc resolves only the query base token
against the tree model and returns typed base-resolution packets to OxCalc; it
does not parse formula text, construct private span keys, or build reference
carriers locally. Focused bridge tests exercise `=SUM(@CHILDREN)`, `=SUM(.*)`,
`=SUM(base.@CHILDREN)`, and `=SUM(base.*)` with no DnaTreeCalc-local parser or
carrier shim; OxCalc rewrites the source handed to OxFml, returns the
`TreeFormula`/`TreeCalcReferenceCollection::ChildrenV1` carrier, and the live
OxCalc/OxFml/OxFunc path publishes the sum of the child values.

The first JSON-backed active slice is now
`references/children-raw-active` with workspace fixture
`workspaces/children-raw-active`. The Rust test runner loads that active theme,
submits the workspace unchanged through `LiveOxCalcTreeBridge`, and asserts only
the supported focused raw children formulas:
`=SUM(@CHILDREN)`, `=SUM(.*)`, `=SUM(base.@CHILDREN)`, and `=SUM(base.*)`. It
asserts published values and dependency membership, not parser output, and it
does not translate strings to carriers in DnaTreeCalc.

The authored `references/set-membership` theme remains `pending`: it contains
broader selectors beyond the current public prebind slices. The first W004
JSON-backed active ordered-selector slice is now
`references/ordered-raw-active` with workspace fixture
`workspaces/ordered-raw-active`. The Rust test runner consumes OxCalc's public
ordered-selector query packets for `@PRECEDING`, `@FOLLOWING`, `@ANCESTORS`,
and `Base.**.Margin`; DnaTreeCalc resolves only the base/member facts and feeds
them back to OxCalc. It asserts published values and dependency membership, not
parser output, and it does not construct reference carriers locally. Do not
treat either active raw slice as completion of W004 set-membership.

The remaining raw-formula blocker is narrowed in bead `dtc-osq.2`: unsupported
families include walk-up and dotted names such as `=A+3`, table structured
references, node-as-function calls, reference literals, dynamic references,
cross-workspace and alias/base-token variants beyond the focused active query
packets, traversal-bound policy, and profile-gated syntax.

## Runner contract

When the `OxCalcTree` bridge exists (W002), a Rust runner consumes this corpus:

1. **resolution** — build the workspace tree as a structural snapshot, bind `reference` from `caller`
   under `profile` via the bridge, assert resolved node / `engine_ref` / `Unresolved` / reject matches `expect`.
2. **classification** — assert the bound reference's cardinality / result-kind.
3. **profile** — assert accept/reject (and INDIRECT parse-interpretation) under each profile id.
4. **membership / table / dynamic / edit** — execute the typed bridge operation for the family and assert ordered members, table target shape, CTRO dependency outcome, or post-edit rebinding consequence.
5. **import** — drive the importer; assert the produced node set + stub formulas + aliases.
6. **value-equivalence workspaces** — emit a replay bundle → OxXlPlay constructs + observes Excel →
   OxReplay diffs. Excel comparison / witness governance is OxReplay's; never duplicated here.

Integration is **file/CLI-based** (JSON in, structured results out — OPERATIONS §6). The bridge
contract is OxCalc's `docs/spec/core-engine/CORE_ENGINE_OXCALCTREE_CONSUMER_INTERFACE_AND_HOST_CONTRACT_V1.md`;
the runner/validator idiom follows `CORE_ENGINE_TEST_VALIDATOR_AND_RUNNER_CONTRACT.md`. The runner
crate's home is deferred to the W002 workspace layout (likely a workspace member, e.g. `crates/treecalc-corpus-runner`).

---

## Validating today

```
pwsh docs/test-corpus/tools/validate-corpus.ps1
```

Parses every JSON file and checks invariants: known `kind`; unique `id`; resolution `caller`/`target`
exist in the named workspace; profile ids/verdicts are known; import cases are well-formed. This is
the acceptance check for corpus-only beads until the Rust runner exists. **No Python** (OPERATIONS §6
tooling rule); pwsh is the convenience layer, Rust is the durable one.

---

## Extending the corpus

Add a case = append an object (with `id`/`name`/`spec`/`kind`) to the right theme file, or add a
`workspaces/<id>.json`. Keep one assertion per case; cite the spec section; re-run the validator.
Full reference coverage lands with **W004**; full import coverage with **W008**; value-equivalence
workspaces and the replay path with **W009**.
