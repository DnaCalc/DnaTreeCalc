# DNA TreeCalc — Operations

How we work in this repo. Deliberately lean: trust the planning surfaces and the bead review step, not ceremony. If a rule here isn't earning its keep, cut it.

This doctrine is the simplified template for the DnaCalc family; the intent is to back-patch it into Foundation and roll it out to sibling repos.

## 1. Precedence

When guidance conflicts, this is the order of authority:

1. **Foundation doctrine** — program-level architecture, profiles, conformance (read-only here; we don't edit it from this repo).
2. **CHARTER.md** — what we're building and why.
3. **The Spec** (`docs/SPEC.md` and the set it indexes) — requirements, design, and top-level planning truth.
4. **This OPERATIONS.md** — how we work.
5. **Worksets** (`docs/WORKSET_REGISTER.md`) — large planned work areas.
6. **Beads** (`.beads/`, via `br`) — atomic execution truth.

CHARTER and OPERATIONS are stable; the Spec evolves with the design; worksets and beads are working surfaces.

## 2. The four planning surfaces

```
Spec  ──────────►  Worksets  ──────────►  Beads
(what & why,       (large work areas,     (atomic units,
 design truth)      W### + register)       br-managed)
       ▲                                       │
       └───────────── Handovers ◄──────────────┘
        (cross-repo asks; can spawn beads directly)
        Issues ─────────────────────────────────► beads (direct)
```

- **Spec** owns requirements/design/planning truth.
- **Worksets** partition ambition into large pushes.
- **Beads** are the atomic, reviewed, committed units of execution.
- **Handovers** coordinate across repos and can generate beads directly.
- **Issues** become open beads directly.

## 3. The Spec

The Spec is the combined requirements + top-level planning + design document **set**, indexed by a single doc: [`docs/SPEC.md`](docs/SPEC.md). CHARTER and this OPERATIONS both point to it.

- It is a structured set (here: `docs/model/`, `docs/interop/`, `docs/ux/`), not one monolithic file.
- `docs/SPEC.md` is the index and reading order; every spec document is reachable from it.
- The Spec is edited freely as the design evolves — it is the design truth, not a frozen contract.
- Where the Spec defines behavior, it gives enough **checkable shape** — concrete examples, expected outputs, Excel-comparison points, or named properties — that useful tests and review checks can be written as the implementation lands. The aim is clarity that supports work, not an evidence bureaucracy.
- Beads that change behavior must update the Spec documents they touch (see §5).

## 4. Worksets

Worksets are **large planned work areas** — a big push for an area, not an atomic task. A sizeable area (e.g. UX) may be two or three worksets. They live in [`docs/WORKSET_REGISTER.md`](docs/WORKSET_REGISTER.md), one living register.

**Naming:** sequential `W###_short_name` (e.g. `W007_meta_nodes_and_formatting`).

**Lifecycle:** `OPEN → IN PROGRESS → CLOSED`, recorded in the register only as a coarse human scan. The register is a roadmap and work history; it is **not** the live execution board. Once the bead store exists, adding a workset to the register means creating its epic bead in the same change; `OPEN` means that epic exists but is not yet underway. The only exception is the bootstrap workset that creates the bead store. `.beads/` owns live truth (`br epic status`, `br ready`, `br list --status in_progress`), and a fresh agent should trust `br` for what is actually ready or underway.

**Each register entry carries:** id, title, one-paragraph purpose, `depends_on`, the Spec sections it realizes, a closure condition (observable state proving done), initial epic lanes, a **verification line** (which gate tiers apply and any obvious scaffolding needed — see §6), and the coarse status.

**No ceremony:** worksets are defined, tweaked, re-scoped, and re-sequenced freely. They are planning containers, not execution-state objects. When adding or materially changing a workset, keep its epic bead aligned. Add child/rollout beads when the path is ready to make explicit (§5).

**Closing a workset includes a housekeeping pass** (§9): bring what the workset touched to a known, in-its-place state — keep, mark, or delete — so the repo stays clear.

## 5. Beads

Beads are the **atomic work units**. They are always and only managed with the `br` tool; `.beads/` is the execution-truth store and is never edited by hand. The bead store is initialized once per repo (`br init`).

Beads carry priority and dependencies, and must hold enough detail to be executable when read together with their workset. A bead states: one reviewable outcome; the smallest useful **acceptance check** — a command, test, click-through, or observation that proves the outcome; its parent epic; real dependencies; and the truth surfaces (Spec docs, etc.) it touches.

**Epic and rollout beads.** Every registered workset has a top bead: its **epic**. The epic title/body should carry the `W###` id so it is easy to pair with the register. Child and rollout beads are linked under that epic using `br` (`--parent <epic-id>` or the equivalent graph relation), so the epic cannot honestly close while its children remain open. Some epics expand directly into child beads; others start with a **rollout bead** whose job is to create/refresh the next child beads once enough context exists (complete when the epic has a believable ready path and the next beads exist explicitly, not just in narrative). Both patterns are fine if the graph stays explicit.

### The bead loop

Run beads sequentially, or in parallel where dependencies allow. For each bead:

1. **Pick** a ready bead (`br ready`; `br show <id>`).
2. **Mark in progress** (`br update <id> --status in_progress`).
3. **Plan** (non-trivial beads only) — note the approach in the bead before coding; skip for small, obvious work.
4. **Do the work** — implement the one outcome; update the Spec docs it touches. Prefer writing or extending a real check first where that's natural; the test suite becomes the contract over time.
5. **Verify, then review.** Run the relevant checks that exist for the touched area (build / test / lint / format once the workspace exists; see `AGENTS.md` "Build, test, verify"). Then **review with fresh eyes**: re-read or use the feature as if new — hunt for blunders, mistakes, oversights, omissions, logical gaps, misconceptions, bugs; rework until a fresh pass raises nothing material. UX/feature beads add a **click-through pass** on the running build; infra/doctrine beads a careful **read-through**. File anything out-of-scope you discover as a new bead before closing. Never weaken or skip a real check to get a bead closed — a wrong check is its own bead.
6. **Commit** once the checks/review are clean. The message references the bead and describes what was actually done.
7. **Close** (`br close <id> --reason "..."`). Close only when the outcome exists, the useful check/review has happened, touched truth surfaces are updated, and newly-discovered work is back in the graph — not because "enough progress happened." Keep close reasons practical: name the decisive check or observation, not a repetitive checklist.
8. **Next.**

**The flywheel.** Each bead should leave the repo's verification surface *stronger* than it found it: behavior added comes with the test or check that proves it, so the next bead is cheaper to verify and safer to change. Well-structured context — Spec, tests, scripts, the bead graph — compounds; that compounding is what lets the agent move fast without breaking things.

### Bead types

The type only changes what "review" and "evidence" mean in practice, not the loop:
- **Feature** — a user-observable outcome; review includes a click-through pass.
- **Infrastructure** — no direct user-observable outcome; review is a read-through.
- **Doctrine** — documentation/spec; review is a read-through against intent.

### `br` tooling

`br` is the only mutation tool; `bv` is for graph-aware inspection. Direct edits to `.beads/` are prohibited. Agents use only non-interactive inspection commands.

```
br ready                                  # list ready beads
br list --status in_progress              # see active live work
br epic status                            # scan epic/workset execution state
br show <id>                              # inspect a bead
br create --title "..." --type epic --priority 2
br create --title "..." --type task --priority 2 --parent <epic-id>
br update <id> --status in_progress
br dep add <id> <depends-on-id>
br close <id> --reason "..."
```

## 6. Verification and anchoring

Pass/fail gates are only as good as the specs behind them. We write specs and worksets so gates are *embeddable*, then flow them through the beads. Verification is **layered** and applied **as appropriate to the work** — never a blanket sweep.

### Gate tiers

1. **Local checks (normal floor).** Build, test, lint, format — see `AGENTS.md` "Build, test, verify". Run the checks that exist and matter for the touched area. During bootstrap or docs-only stages, the local check may be a read-through, link/register scan, or `br` sanity check; once the Rust/WASM workspace exists, cargo/trunk checks become the normal floor for code-affecting beads.
2. **Excel anchor (where behavior is Excel-defined).** DNA Calc's canonical reference is Excel. Where a behavior is meant to match Excel, the gate is "matches Excel." Scope is decided by the **Excel-alignment boundary** (Spec `model/CORE_MODEL_SPEC.md` §5): the Excel-aligned surface (function semantics, values, number formats, dates, error codes) is Excel-anchored; the novel tree/reference/skin surface has no Excel counterpart and is pinned by spec-defined cases instead. Anchor against an Excel-derived fixture for single values, or — for workspace-level behavior — through **OxXlPlay** (constructs + observes Excel) and **OxReplay** (diffs and governs the comparison). Excel comparison is implemented once in OxReplay; never reimplemented here.
3. **Formal (Lean / TLA+).** Formal anchoring has a larger role in DNA Calc than a gate alone — see "Formal anchoring is a standing aim" below. As a *gate*, a proof or model is used where a property is load-bearing and better pinned by proof than by examples. It is not a uniform, continuously-enforced layer and need not gate every workset. Much engine-level formal weight sits in OxCalc (coordinator / epoch / invalidation); TreeCalc's own candidates are its reference / scope / set-operator and recalculation semantics.

"As appropriate" is the governing rule for *gating*: a workset declares which tiers gate it. Don't force an Excel gate onto novel-surface behavior with no Excel meaning; don't force a proof where cases suffice. (The standing aim below is separate from gating and always applies.)

### Formal anchoring is a standing aim

A prime motivator for DNA Calc is to make Excel's *implicit* behaviour *explicit* and anchor it — where possible — in formal descriptions, so the calculation stack (engine → formula evaluation → function calls) stays correct as it changes. That aim shapes **how we build**, even when it isn't gating:

- Keep semantics **explicit and formally-traceable**, not ad-hoc — so a Lean/TLA+ counterpart can grow to meet the code later, even where none exists yet.
- Where a formal description exists, treat it as a reference the implementation tracks: change the implementation freely, but keep it answerable to the model.
- Don't write code, specs, or process in a way that quietly forecloses this. The formal layer becomes more prominent over time; leave the door open.

This usually doesn't block a bead — but the docs must not treat it as a minor afterthought. It is part of why the project exists.

### Gates flow spec → workset → bead

- **Specs give checks something to bite on** — examples, expected outputs, Excel-comparison points, profile-rejection cases, or named properties. Write enough of these to guide implementation and tests; don't stall coding to over-spec every corner before the first slice exists.
- **Worksets name the verification shape** — which tiers apply and any obvious scaffolding (test harness, Excel fixture corpus, `verify-workspace` plumbing, replay-bundle emitter). Build that scaffolding early when it is needed for real progress, not as an after-the-fact paperwork lane.
- **Beads inherit a concrete check** — a bead's acceptance check is the useful runnable or observable proof for its one outcome. "Verify a lot of things" is not a bead.

### Tooling & scaffolding discipline

- Verification scaffolding is work when it removes friction from real feature work: a workset that adds Excel-anchored behavior should plan the smallest fixture/harness path that lets agents test the behavior repeatedly.
- **Tooling language:** this is a Rust repo, and tooling is **Rust-first** too — scaffolding, test runners, and Excel-drivers in Rust are all fine. PowerShell (`pwsh`) for convenience launchers/orchestration. **No Python for casual tooling** (a logged exception — scope/owner/sunset — only if ever truly warranted). (This relaxes Foundation §6.4's .NET-first tooling mandate, which predates the family's Rust-heavy reality — flagged as a back-patch candidate.)
- **Two Excel paths, both legitimate.** (a) A fast, direct, local Excel validation/comparison runner in Rust for high-performance iteration — like OxFunc's integrated runner; ephemeral, not a durable artifact. (b) The canonical OxXlPlay (construct + observe) + OxReplay (diff + govern) path for durable, family-comparable, witness-governed verification. Use (a) freely for speed; use (b) for canonical results. The one thing not to duplicate is OxReplay's *canonical comparison and witness governance* — don't rebuild that here.
- **Cross-tool integration is file/CLI-based** — schemas, traces, manifests (the OxXlPlay / OxReplay seams). Don't couple any other way.

## 7. Handovers

Handovers are the cross-repo coordination channel — and a second way to generate beads (skipping worksets). All coding/other work still happens as beads; a handover can spawn beads directly.

- A handover is **one file** in `docs/handovers/`, named `HANDOVER_<TARGET>_<short_topic>.md`.
- It holds the **request** at the top (what's needed, why, relevant evidence, the exact ask) and a one-word **status** line (`Open` / `Responded` / `Done`).
- The **response is appended to the same file** — and the response is the completion. No separate response doc, no acknowledgment doc, no receipt doc, no CSV register.
- A landed handover (incoming) can be turned into an open bead directly (§8).
- Keep it conversational and short. The point is a simple back-and-forth, not a sign-off protocol.

A useful lightweight header is:

```
Status: Open
Target: <repo>
Ask: <one sentence>
Context: <why this matters>
Evidence: <links / repro / spec section, only if useful>
```

## 8. Issues

A reported issue becomes an **open bead directly** — `br create … --status open` with the issue as the bead body, linked to a workset/epic if one fits. No separate issues register or `ISSUES.md`. If the issue needs another repo, raise a handover instead (§7).

## 9. Housekeeping

Housekeeping keeps the repo *clear*: every artifact in its right place, with a known status. Its first job is **staying on top of what's under the project umbrella** — what's live, what's parked, what's superseded — not deletion for its own sake. Stale or ambiguous context misleads agents and humans, so the aim is *ongoing clarity*, refreshed often enough that doubt never piles up.

**Trigger:** a housekeeping pass at the **close of every workset** (part of closing the epic), and any time things have drifted. This is the per-workset rhythm, as the bead loop (§5) is the per-bead rhythm — doing it regularly is what keeps it light, because you settle each artifact's status while it's still fresh.

Walk the surfaces the workset touched — Spec docs, the workset register, handovers, prototypes and fixtures, scripts, code — and bring anything that's drifted to a known state by taking **one of three actions** (moving something to the right place is often the whole job):

- **Keep & place** — it's live: put it in the right location and make sure `docs/SPEC.md` and the register reflect it.
- **Mark** — it's parked, exploratory, or partially superseded: leave a short status note so its state is unambiguous (e.g. "parked — revisit after W0XX", or "superseded by Y *except* the Z part, which still needs folding in — bead filed"). A partial supersession gets a note that the remainder must be completed.
- **Delete** — it's genuinely dead or fully superseded: delete it, and don't be timid — git history is the safety net, and a confidently-dead doc lingering behind a "DEPRECATED" banner is just noise. Repair any references the delete or rename leaves dangling.

**The balance.** Deletion is triggered by *superseded / dead / wrong* — **not** by "unreferenced right now." Half-formed ideas and not-yet-wired prototypes are often unreferenced and still valuable; don't sweep them out. When you're confident something is dead, delete it freely. When you genuinely doubt whether something is still load-bearing, treat the doubt as a signal to **resolve** it — and resolving can mean *mark it, move it, confirm it, or defer it to the next pass*, not necessarily delete it now. Caution under real doubt is fine; the regular cadence means doubt gets settled soon rather than buried. The thing we won't tolerate is *drift* — an artifact whose status nobody knows.

During housekeeping, reconcile the workset register with the bead graph at the workset/epic boundary: every non-bootstrap register entry should have a matching epic bead; every active workset epic should have a register entry; and obvious coarse-status drift between register and epic should be fixed on the stale side. Do not duplicate `br`'s child/epic consistency checks in the register sweep — child closure, blockers, readiness, and in-progress state belong to `br`.

If a repo checker exists, treat it as a housekeeping aid: link health, register shape, workset/epic pairing, and obvious stale pointers. It does not own readiness, blocker state, or closure truth; `br` does.

## 10. Relationship to the other root docs

- [`AGENTS.md`](AGENTS.md) — how an agent should operate in this repo (read-only sibling access, no writes outside the repo, pointers here).
- [`CHARTER.md`](CHARTER.md) — mission and program/repo context.
- [`README.md`](README.md) — one-screen orientation and pointers.

Keep this document lean. New process rules earn their place only by removing real, recurring pain.
