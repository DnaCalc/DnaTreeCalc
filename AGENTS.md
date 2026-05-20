# AGENTS.md — DNA TreeCalc

Operating instructions for agents working in this repo. Minimal by design.

## Start here

Load context in this order before doing work:

1. [`CHARTER.md`](CHARTER.md) — what we're building and why; place in the DNA Calc program.
2. [`OPERATIONS.md`](OPERATIONS.md) — how we work (spec, worksets, beads, handovers, issues).
3. [`docs/SPEC.md`](docs/SPEC.md) — the spec/design document set (requirements, design, planning).

When picking up execution: read `docs/WORKSET_REGISTER.md` as the roadmap, then use `br epic status`, `br ready`, and `br list --status in_progress` for live state. If `br` is not initialized yet, the repo is still in bootstrap; follow W001.

## Repo boundary

- You **may read** sibling repos under `..\` (e.g. `..\OxCalc`, `..\OxFml`, `..\OxFunc`, `..\OxXlPlay`, `..\OxReplay`, `..\Foundation`) for context and contracts.
- You **must not write** anything outside this repo's folder. No edits to sibling repos, ever. To request work in another repo, raise a handover (`OPERATIONS.md` §7).

## Source of truth

- Foundation is read-only doctrine; sibling-repo specs are owned by those repos. Treat anything you read elsewhere as observed state, not local truth.
- This repo's own truth precedence is in `OPERATIONS.md` §1.

## Execution discipline

- All coding and substantive work happens as **beads**, managed only with `br`. Never hand-edit `.beads/`.
- Follow the **bead loop** (`OPERATIONS.md` §5): pick ready bead → (plan if non-trivial) → do the work → **verify** (run the relevant checks that exist for the touched area) + **fresh-eyes review** (find and fix blunders/omissions/bugs; UX beads add a click-through, infra/doctrine a read-through) → commit → close → next.
- Update the Spec documents a bead touches as part of that bead.
- File out-of-scope discoveries as new beads before closing the current one.
- Never weaken, skip, or delete a real test/check to make a bead closeable. If a check is wrong, fix it as its own bead. Don't claim done without running the relevant available checks, and don't fabricate evidence.

## Build, test, verify

The commands that define the normal code floor once the Rust/WASM workspace exists. Run the relevant ones after changes that touch code or buildable artifacts, and make sure they pass before closing that bead. For pre-bootstrap or docs-only work, use the useful available checks instead: careful read-through, link/register scan, `br` sanity check once initialized, or another observation that actually proves the change.

| Check | Command |
|---|---|
| Build | `cargo build --workspace` |
| Test | `cargo test --workspace` |
| Lint | `cargo clippy --workspace -- -D warnings` |
| Format | `cargo fmt --check` |
| Web build (WASM shell) | `trunk build` (or `wasm-pack build`) |

These are wired as the Rust workspace lands (worksets W001–W002); keep the list current as the build system grows. Treat the **test suite as the contract** — prefer adding or extending a test when it will keep proving the behavior. If the work you're doing has no useful check, add the smallest one that will matter; don't invent paperwork just to tick a box.

Beyond these local checks, **Excel-anchored** gates (directly or via OxXlPlay/OxReplay) and — where a property earns it — **formal** (Lean/TLA+) gates apply *as appropriate to the work*. The workset declares which apply; see `OPERATIONS.md` §6 "Verification and anchoring".

## Safety

- Never run destructive git or filesystem operations without explicit instruction.
- Do not skip git hooks or bypass signing unless explicitly asked.
- Keep changes scoped to the bead in hand.

## Nested context

Crate- or area-level `AGENTS.md` files may add local build/test/convention notes; this root file governs the repo. Keep each one short and current — context is infrastructure: well-structured context compounds, stale context misleads.

That's it. Detail lives in `OPERATIONS.md`; don't duplicate it here.
