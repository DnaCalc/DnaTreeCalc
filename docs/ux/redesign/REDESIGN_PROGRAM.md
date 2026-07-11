# DNA Calc Front-End Redesign — Program Charter

Status: ACTIVE v0.1 · 2026-07-12 · owner-ratified decisions D1–D4 recorded below.
Companion docs: [STRAND_DESIGN_LANGUAGE.md](STRAND_DESIGN_LANGUAGE.md) ·
[SKIN_IR_GAP_REGISTER.md](SKIN_IR_GAP_REGISTER.md) · [MECHANISMS.md](MECHANISMS.md).
Spec register: [SHELL_SPEC.md](SHELL_SPEC.md) v0.9 · [BENCH_SPEC.md](BENCH_SPEC.md) v0.9 ·
[NOTEBOOK_SPEC.md](NOTEBOOK_SPEC.md) v0.3 · [SHEET_SPEC.md](SHEET_SPEC.md) v0.2 ·
[MODEL_SPEC.md](MODEL_SPEC.md) v0.2 · [ATLAS_SPEC.md](ATLAS_SPEC.md) v0.2 — draft specs
harden to ratification at their phase kickoff; SHELL/BENCH are ratification candidates now.
Deferred-by-design: [PARITY_TRUST_UX.md](PARITY_TRUST_UX.md) (traces only, outside all work lists).
Visual vision board (v0.1): claude.ai artifact `dnacalc-design-vision` (private; regenerate from this doc set if lost).

## 1. Thesis

Keep the calculation world people already trust — Excel's formula language, function semantics,
file format — and let them **change the viewpoint instead of changing tools**. Views are
projections, not products: one shell, two model profiles, four stage identities, one theme system.
The redesign is built **strictly on the Skin IR** (`dnacalc-skin-ir`); where the vision needs more
than the IR expresses, we file a numbered ask (G1–G10) — we never shim.

This program supersedes the presentation layer only. Engine, host-core, Skin IR layering,
doctrine (three-layer CALC/OVERLAY/ANNOTATION separation, three mutation surfaces,
derive-don't-store, no shims, fail-until-fixed testing) all stand.

## 2. Ratified decisions (owner, 2026-07-12)

| # | Decision | Ruling |
|---|---|---|
| D1 | Skin stack | **Rust/Leptos chrome + canvas stages.** New crates; Skin IR stays serializable-first so a TS/remote skin remains possible. |
| D2 | Shipping order | **Bench (OneCalc) → Notebook → Sheet → Model.** S0 foundations first in every case. |
| D3 | Platform | **Strict parity from day one.** Every UI bead lands and is verified on browser WASM and Tauri desktop in the same bead. |
| D4 | Stage lineup | **Sheet · Model · Notebook · Atlas** adopted as proposed. Canvas/diagram are layouts inside Model, not stages. Companions fold into shell chrome. Themes replace the 16-skin concept. "Atlas" is reused for the probe stage; the historical ATLAS lens-suite usage retires. |
| D5 | Repos & tiers | **DnaOneCalc merges into this repo (S0); crate tiers with enforced dependency gates** — the OneCalc OxCalc-free rule becomes the per-commit F-gate. Delegated decision under owner authorization; details in *Repo consolidation & crate tiers* below. |

Settled by the vision unless challenged: Strand tenets and palette roles · shell anatomy
(Mast / Bridge / Registry / Stage / Inspector / Strip) · profiles-as-identities with stages shared ·
the skin consolidation map (§4) · strict Skin-IR-only layering with the G1–G10 ask register.

## 3. Product map

| Host / profile | Home stage | Companion stages | Identity badge |
|---|---|---|---|
| **DNA OneCalc** — single-formula instrument | **Bench** | — | Cyan `dot-long` (cyan `#3DB6CF` + petrol) |
| **DNA Calc · Rich Tree** — premium modelling profile | **Model** (outline · canvas · diagram layouts) | Notebook · Atlas | Sage `long-dot` (sage `#71A08A` + forest) |
| **DNA Calc · Excel-strict** — workbook/sheets/grid/names | **Sheet** | Notebook · Atlas | Teal `twin` (teal `#318995` + petrol) |
| **DNA Calc · Hybrid** (future) | Model with grid-region node cards | Notebook · Atlas | composition, not a new stage |

Axis rules:
- A **stage is a projection**: switching is re-projection, never re-load; selection and focus survive.
- **Profiles gate capability, stages gate layout.** Sheet exists only where a grid exists; Notebook and Atlas exist everywhere.
- **Layouts inside Model are overlay data** — positions never touch calc.
- **OneCalc and Calc share the Bridge verbatim** — one formula experience everywhere a formula is edited.
- Tree scoping honors the model spec: top-level nodes carry workbook-like scope; second level acts sheet-like; deeper levels extend naturally (`docs/model/CORE_MODEL_SPEC.md`).

## 4. Where the sixteen registered skins go

Every capability survives; almost none survive as a separate skin
(census: `src/dnatreecalc-host/src/app/registry.rs`).

| Today | Destination |
|---|---|
| Capture, Tree, Ledger, Sheet-on-tree | **Model** stage (structure-by-typing entry, fold/reshape, cleave bar, table cards) |
| Flow | **Atlas** stage (dependency traversal, reading-head replay, explain) |
| Bench (scenarios/sweeps) | **Notebook** scenario chips + Inspector compare |
| Transport (revision DAG) | Shell **Timeline** drawer, available in every stage |
| Notebook, Workbook | Carried forward as **Notebook** and **Sheet** |
| Lens, Console companions | Become chrome: **Inspector** and **Strip** |
| Triple editor, Formula tree, Outline table, Value board, Dependency inspector | Retired |

## 5. Program phases

Strict parity (D3) applies from S0: the parity CI job (browser build + headless browser tests +
Tauri build) is part of S0's definition of done, and every subsequent bead ships against it.

- **S0 — Foundations** (this repo)
  - DnaOneCalc subtree import + `dnaonecalc-*` → `dnacalc-bench-*` renames + the three
    dependency gates (F/P/T0) as pinned tests (D5).
  - Strand token crate (`--dna-*` custom properties, light/dark/high-contrast; see STRAND doc §6),
    including per-hue `-ink` text variants and **unit-tested contrast assertions over every
    sanctioned foreground/background token pair** (STRAND §2.1).
  - Shell skeleton: Mast, Bridge, Registry, Stage host, Inspector, Strip; stage switcher; command deck stub.
  - Bridge v1 over `dnacalc-formula-ux-core` (token runs, diagnostics, completions, signature help).
  - File gap-register asks G1–G10 into `docs/ux/stack-requirements/HOST_AND_SKIN_IR_REQUIREMENTS.md`
    and upstream lanes where owned there (see SKIN_IR_GAP_REGISTER.md for owners).
  - Parity CI job green.
- **S1 — Bench** (in-tree after the D5 import; the OneCalc product, F-gate enforced)
  - OneCalc rebuilt on the Strand shell: Bridge + X-Ray drill + array result windows
    (all already in the OneFormula IR surfaces), formatting/CF/locale controls, twin-oracle strip.
  - Extensions manager v0 + feed instruments (needs G7 minimal slice).
- **S2 — Notebook** (workbook profile first, per the B1 route)
  - Narrative blocks (G6), name-first authoring, cell/table entries, scenario chips.
  - Atlas skeleton: structure map over sheets/names + calc HUD from `CalcRunProjection`.
- **S3 — Sheet**
  - Canvas grid renderer (Canvas2D tiles + DOM overlay editor; RenderPlan-style geometry tests).
  - Grid interaction pack (G3) and viewport/LOD pack (G4); semantic zoom v1 (names-over-blocks).
  - Atlas gains grid dependency traversal (G9).
- **S4 — Model**
  - Prerequisite: RichTree session extraction into Leptos-free `dnacalc-host-core`
    (the `DocumentSession::RichTree` seam is currently empty).
  - Outline + node cards, capture-by-typing, tables-in-node cards, reference X-Ray with
    resolution-path replay, per-node styling (needs G2), canvas/diagram layouts (G10 optional).
- **S5 — Hybrid groundwork + completeness**
  - Grid-region node card proof (Sheet surface hosted in a Model card), keyboard atlas
    completeness pass, theming polish, published/locked Notebook reading mode.

### Repo consolidation & crate tiers (D5 — delegated decision under owner authorization 2026-07-12)

**The OneCalc boundary is architectural, not geographic** (owner, 2026-07-12): DnaOneCalc
exists to prove that the single-formula surface stands on **OxFml + OxFunc alone — never
OxCalc** — forcing the OxFml/OxCalc separation to stay real. A repo boundary only blocks
path-dep declarations; a dependency-graph gate asserts the whole resolved graph on every
commit. So the forcing function moves from repo geography into pinned tests, and the repos
consolidate:

**D5 (decided):** DnaOneCalc's live tree merges into this repo early in S0 (git subtree, history
preserved; `src_archive_ref` stays behind in the archived repo). The DnaOneCalc repo is then
frozen read-only with a pointer here. This repo is the program home and will be renamed later
(owner note; no action now). Redesign sessions work in one workspace, one parity CI, one bead
register going forward.

**Crate tiers** (dependency law, verified by fact-check of current manifests):

| Tier | May depend on | Crates |
|---|---|---|
| T0 · Protocol | serde only — no engines, no UI framework | `dnacalc-skin-ir` |
| TP · Presentation | T0 + Leptos/web-sys — **no Ox\* crate, ever** | `dnacalc-strand`, `dnacalc-shell`, `dnacalc-bridge`, `dnacalc-stage-{sheet,model,notebook,atlas}`, app UI composition |
| TF · Formula tier | T0 + OxFml + OxFunc — **OxCalc forbidden** | `dnacalc-formula-ux-core`, `dnacalc-extension-host-core`, `dnacalc-bench-host` (from `dnaonecalc-host`/`-core`) |
| TC · Calc tier | T0 + OxCalc (brings OxFml/OxFunc) | `dnacalc-host-core`, worker runtime, Calc app roots |

Apps are thin composition roots: **Bench app** = TF host + TP presentation (the OneCalc
product, browser + Tauri); **Calc app** = TC host + TP presentation. The Bridge being TP-pure
is what lets one formula workbench serve both products.

**Enforced gates** (pinned tests in the style of the existing no-Leptos `cargo tree` gate on
`dnacalc-skin-ir`):
- **F-gate (the OneCalc forcing function):** the Bench app's resolved dependency graph contains
  no `oxcalc*` crate. Continuous, per-commit, stronger than the old repo boundary.
- **P-gate:** every TP crate's graph contains no `ox*` crate (skins speak Skin IR only —
  existing doctrine, now asserted per crate).
- **T0-gate:** `dnacalc-skin-ir` stays free of Leptos and engines (exists today).

Migration outline (S0 bead): subtree-import DnaOneCalc → `src/` + `docs/onecalc/`; retarget its
path deps to workspace deps; rename `dnaonecalc-*` → `dnacalc-bench-*` in place; land the three
gates; OneCalc's charter mission (Twin Oracle Workbench, OxFml/OxFunc proving) carries into the
Bench tier unchanged.

## 6. Relationship to existing plans

- **THREE_FRONTENDS_PLAN.md** stands. B3 (CLI/MCP) is untouched and becomes the agent
  backbone of mechanism 20. B1's functional scope is absorbed into the Notebook stage; the K
  (workbook) route's functional scope is absorbed into the Sheet stage. The
  `FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` verb map, error-presentation table, and H-track
  host-core beads remain normative; its N/K *visual* design and the open F.3 visual-identity item
  are superseded by this program + STRAND.
- **ATLAS lens suite** (`docs/ux/skin-suite/`): doctrine (spine laws, keybinding grammar,
  continuity) carries forward; the suite as a product surface retires per §4. The name "Atlas"
  now refers to the probe stage.
- **W011 / W062**: unchanged. This program consumes the landed W062 R5/R6 verbs and the
  W011 crate architecture (`dnacalc-skin-ir` / `dnacalc-host-core` / skin layer).
- **Extension architecture** (`docs/ux/EXTENSION_ADAPTER_ARCHITECTURE.md`): unchanged;
  this program adds only the projection asks (G7) and the manager/instrument UI.
- **DnaOneCalc**: the repo merges here under D5 and is then archived. Its charter mission —
  the single-formula proving host that keeps OxFml/OxFunc honest without OxCalc — continues
  unchanged as the Bench tier, with the F-gate as its enforcement. The host progression ladder
  reads the same; only the geography changed.
- **OxXlPlay / OxReplay / U-ORACLE (parity evidence)**: the infra lanes continue unchanged as
  CI/proof machinery. The *product* parity UX — surfacing evidence to users uniformly across
  all surfaces — is **designed-for but deferred out of S0–S5** (owner direction 2026-07-13):
  design traces, state vocabulary, reserved slots, and the unfiled P-register of future IR
  asks live in [PARITY_TRUST_UX.md](PARITY_TRUST_UX.md). No S-phase bead or acceptance may
  include parity UI; reserved slots render nothing.

## 7. Next actions

1. ~~Draft the spec set~~ — done 2026-07-13 (see spec register above). Next gate: **owner
   ratification of SHELL_SPEC + BENCH_SPEC** (v0.9 → v1.0); draft specs harden at their phase
   kickoffs.
2. File G1–G10 asks in the requirements ledger with the owner lanes from the gap register,
   plus the S1 asks named in BENCH_SPEC §8.
3. S0 bead graph (beads-workflow) once SHELL_SPEC is ratified — D5 import bead first, then
   strand/shell/bridge/parity-CI beads per SHELL_SPEC §10 acceptance.
4. BENCH_SPEC governs S1 scope: what stays Bench-local vs graduates to shared TP crates.
