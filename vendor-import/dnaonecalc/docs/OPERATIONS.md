# DnaOneCalc Operations

## 1. Purpose
This document defines how `DnaOneCalc` is operated day to day.

It is intentionally concise.
For the detailed bead method, tooling usage, rollout template, and worked example, see [BEADS.md](BEADS.md).

## 2. Source-Of-Truth Order
Within this repo, precedence is:
1. `docs/CHARTER.md`
2. `docs/SCOPE_AND_SPEC.md`
3. this file
4. `docs/WORKSET_REGISTER.md`
5. `.beads/`

Interpretation:
1. `docs/SCOPE_AND_SPEC.md` owns scope, design, and artifact truth,
2. `docs/WORKSET_REGISTER.md` owns workset truth,
3. `.beads/` owns execution state,
4. this file owns the operating model and behavioral rules for working in the repo.

## 3. Execution Surfaces
Workset and execution surfaces live in:
1. `docs/WORKSET_REGISTER.md`
2. `.beads/`

The register holds the ordered workset set, their meaning, and their dependency shape.
The bead graph holds epics, beads, dependencies, blockers, readiness, in-progress
state, and closure.

These are the planning documents for repo execution.
Once doctrine and planning setup exist, default execution outputs should be
implementation code, test code, and narrowly-scoped spec or seam corrections,
not a growing body of local per-bead notes.

Interpretation rule:
1. worksets are high-level work themes, not execution-state objects,
2. the register does not track `active`, `ready`, `blocked`, or `complete` status per
   workset,
3. a workset is incomplete while it still has open epics or leaf beads in `.beads/`,
4. neither the register nor the bead graph should be treated as a mandate to
   create one document per work item.

Default execution model:
1. use the register to choose the next workset(s) to expand,
2. roll chosen worksets into epics,
3. create some execution beads directly during rollout when the path is already clear,
4. use rollout epics where the child bead set still needs to be discovered or staged
   during execution,
5. let the bead graph own the resulting ready set and dependency tracking,
6. expand early or well-understood work directly into executable child beads rather
   than hiding obvious implementation behind rollout placeholders,
7. close beads only with visible outcome and evidence.

## 4. Cross-Repo Read-Only Doctrine
Agents working from the `DnaOneCalc` repo may read files in sibling repositories under the shared `DnaCalc` root when needed for seam consumption, integration, evidence intake, or architectural alignment.

Those sibling repositories are read-only from the perspective of this repo.
Required changes outside `DnaOneCalc` must be routed through an explicit handoff, prompt, or separate repo-scoped run.

Cross-repo visibility is permission for understanding, not for opportunistic cleanup or silent fixes.

## 5. Bead Mutation Rule
Use `br` to mutate bead state.
Do not edit `.beads/` files directly.

`bv` is supported for graph-aware triage and analysis.
Use only non-interactive robot-style invocations from agent sessions.

## 6. Validation Discipline
Minimum local expectations before claiming meaningful progress:
1. touched docs reflect the new truth,
2. `docs/WORKSET_REGISTER.md` still matches the intended workset sequence and scope
   partitioning,
3. bead state is synchronized with the actual execution state,
4. relevant local checks for the touched area have been run where available.

Bootstrap validator:
- `scripts/check-worksets.ps1`

Interpretation:
1. this script is only a register shape check,
2. it does not report bead readiness, blockers, progress, or closure truth.

## 7. Change Discipline
1. Keep changes minimal, explicit, and reviewable.
2. Do not silently widen OneCalc toward `OxCalc`.
3. Do not claim replay, compare, formatting, conditional-formatting, or extension breadth beyond the retained evidence and admitted scope.
4. Do not substitute documentation rollout for implementation progress.
5. Capability-bearing work should normally land meaningful code plus verification.
6. Local documentation after planning setup should be limited to spec corrections,
   upstream seam handoffs, or necessary reference for behavior that now exists in code.
7. When upstream seams need to change, produce a handoff rather than normalizing local drift.

## 8. Document Count Discipline
This repo intentionally keeps a small top-level doc set.

Default rule:
1. do not create one document per workset,
2. do not multiply status documents when `WORKSET_REGISTER.md` and `.beads/` already hold the needed truth,
3. do not create bead-sized local notes as a default execution output,
4. do not split the bead method across many files unless the repo later grows enough complexity to justify it.

## 9. Root-cause Discipline
When a user-visible bug is rooted in an upstream sibling repo
(`OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`, `Foundation`, ...), fix
it upstream. Do not patch the symptom inside `DnaOneCalc`.

Guarantee:
1. agents must locate the source of the defect, not the location
   where it surfaces,
2. the place a defect surfaces is rarely the place where it should
   be fixed,
3. host-side workarounds for upstream defects are forbidden by
   default, even when they are easy to write and would silence
   the failing test.

Why this matters:
1. workarounds layer permanently — they outlive the original cause
   because nobody removes them once the upstream fix lands,
2. workarounds hide regressions — when the upstream defect changes
   shape (worse, or different) the host code keeps painting over
   it and nobody notices,
3. workarounds widen the host's responsibility surface — every
   workaround is a class of bug that the host now claims to handle,
   which means tests + docs + future agents must reason about it,
4. workarounds confuse audiences — the bug is reported from the
   host but lives upstream; pinning the fix at the right layer is
   how teams stay aligned on which repo owns which class of
   correctness.

Required process when an upstream defect is identified:

1. **Reproduce at the upstream layer.** Write the smallest possible
   test that exercises only upstream surfaces (a unit test inside
   the upstream crate, or a `cargo test -p oxfml_core` style probe).
   This tells the upstream maintainer where to start and is the
   shape of the eventual upstream regression test.

2. **Capture a handoff.** Write a markdown file in
   `docs/handoffs/<concise-slug>.md` containing:
   - the user-visible symptom on the host side,
   - the diagnosis (a token-level / call-level dump showing what
     upstream actually produces vs. what is expected),
   - the expected post-fix behaviour,
   - related cases worth checking,
   - explicit "what not to do" — note that a host-side workaround
     was considered and rejected, with a brief reason,
   - a coordination checklist for what the host will do once the
     upstream fix is in (bump dependency, remove ignored test,
     delete handoff).

3. **Mark the host reproduction `#[ignore = "..."]` with a reason
   string** that names the upstream surface and references the
   handoff file. The reproduction stays in the corpus so that:
   - agents (human or AI) can see the bug is known and tracked,
   - the moment the upstream fix lands, removing the `#[ignore]`
     and re-running the test is the regression gate.

4. **Do not** add host-side gap-fill / fallback / synthesise-the-
   missing-output code paths to silence the failing test. If the
   user-visible symptom is unacceptable until the upstream fix
   ships, the right responses are:
   - escalate the priority of the upstream fix,
   - file a follow-up bead to track the dependency-bump that
     re-enables the test once upstream lands,
   - in the rarest cases, accept the symptom as a known issue
     that the handoff documents.

Acceptable host-side scope (NOT subject to this rule):

* genuine UI-rendering concerns (e.g. how the host displays
  upstream tokens, what colours they get, what aria attributes
  the spans carry) — these are host code by definition,
* genuine host-bundle composition (combining multiple upstream
  outputs into a UI-shaped projection) — but only adding
  structure, never fabricating content the upstream didn't
  produce,
* defensive guards against upstream changing API shape (e.g.
  `Option::is_none` checks, `match` exhaustiveness) — these
  protect against build breakage, not against semantic bugs.

The line between "host display concern" and "host workaround for
upstream defect" is whether the host is **adding structure** to
upstream output (allowed) or **fabricating content** that upstream
should have produced but didn't (forbidden by default).

Reference: the OxFml leading-whitespace tokenizer truncation bug
(handoff initially captured at
`docs/handoffs/oxfml_leading_whitespace_truncation.md`, since
deleted after upstream fix `162f224` shipped) is the canonical
example. The host attempted a `Text`-role gap-fill in
`syntax_runs_from_snapshot` that synthesised the dropped `aaa`
characters. It was reverted; the fix routed upstream.

### 9.1 Catching upstream-contract violations early

The two known violations of the snapshot-tile-source-text
contract (`\n=aaa` leading-whitespace, `= a a` inter-identifier
whitespace) both surfaced because nobody had typed those exact
strings in a test. The general pattern: a contract that the
host implicitly relies on, exercised by only one or two
hand-picked test inputs, missed defects that other inputs would
have surfaced.

Defence: parameterised invariant matrices. For any contract
the host depends on, write a matrix of representative inputs
and assert the contract holds for each, with one
`#[wasm_bindgen_test]` (or one native test) per input so a
failure localises to a specific case. Adding a new input is
one line; the matrix grows with discovered edge cases and
becomes the regression net.

Concrete example: `tests/browser/buffer_integrity.rs` runs the
"snapshot tokens + trivia must tile source text" contract
against a list of formula-shape inputs (well-formed calls,
bare identifiers, leading-whitespace, inter-identifier
whitespace, multi-space, cell-refs, text literals, forced
text, partial parses, ...). Inputs that fail upstream are
`#[ignore]`d with a reason string naming the upstream surface
and the relevant `docs/handoffs/...` file; once the upstream
fix ships, removing the `#[ignore]` is the regression gate.

When you find a new bug rooted in an upstream-contract
violation, add the failing input to the relevant matrix as
part of the same change that captures the handoff. The matrix
is where the test framework grows toward each newly-discovered
class of violation.
