# AGENTS.md — DnaOneCalc Agent Instructions

## 1. Start Here
Read [README.md](README.md) first.

Then follow the local reading order from the README:
1. `docs/CHARTER.md`
2. `docs/OPERATIONS.md`
3. `docs/SCOPE_AND_SPEC.md`
4. `docs/WORKSET_REGISTER.md`
5. `docs/BEADS.md`

## 2. Local Authority
Within this repo:
1. `docs/SCOPE_AND_SPEC.md` is the main engineering authority,
2. `docs/WORKSET_REGISTER.md` owns workset truth,
3. `.beads/` owns execution state,
4. `docs/BEADS.md` defines the local bead method.

## 3. Cross-Repo Rule

Agents working in this repo may **read** sibling repositories under
`C:\Work\DnaCalc` (`OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`, `OxVba`, `OxIde`,
`OxCalc`, `Foundation`, etc.) for context, upstream contracts, reference docs,
and retained evidence.

Agents **must not write** outside this repo.

**Hard prohibitions** (no exceptions, no “quick fixes”, no “while I’m here”):
1. Do not edit, create, delete, rename, or reformat files outside `DnaOneCalc`.
2. Do not run `git add`, `git commit`, `git checkout`, `git reset`, `git stash`,
   `git rebase`, `git merge`, or any other state-mutating git command in a
   sibling repo working tree.
3. Do not run `cargo fix`, `cargo fmt`, code generators, or migration scripts
   that touch sibling repo files.
4. Do not edit a file whose absolute path is not under
   `C:\Work\DnaCalc\DnaOneCalc\`. The path test is the rule, regardless of how
   the file was reached (path-dep cargo workspace, symlink, reference doc).

**This applies even when:**
- The bug is unambiguously in the upstream repo and the upstream fix is small.
- The host-side workaround would be ugly and the upstream fix would be clean.
- OPERATIONS.md §9 says “no host-side workaround for an upstream defect”.
  That instruction is about *not papering over* the issue locally — it is **not**
  permission to reach into another repo. The correct response is a handoff, not
  a sibling-repo commit.

**How to capture cross-repo work instead:**

When the agent identifies that an upstream repo needs to change, the only
sanctioned outputs are *inside DnaOneCalc*:
1. **Write a handoff doc** at `docs/HANDOFF_<REPO>_<TOPIC>.md` (matching the
   existing `HANDOFF_OXFML_*`, `HANDOFF_OXREPLAY_*`, `HANDOFF_OXXLPLAY_*`
   pattern). The doc states the symptom observed in DnaOneCalc, the upstream
   root cause as best understood, the proposed surface change, and a minimal
   reproduction path.
2. **Bead it** under the appropriate workset (see §4) so the handoff is tracked
   to closure.
3. **Report the handoff back to the user** in the answer that surfaces the
   issue, with the doc path and the one-line summary. Do **not** silently
   continue assuming the upstream change will land.

If a host-side mitigation is needed in the meantime, mark it explicitly in code
with a `// SEAM-<UPSTREAM>-<TOPIC>` comment that points at the handoff doc, and
keep the mitigation visibly *temporary* — never inline it into the architecture.

**On discovering the rule has been violated** (including by a previous agent
turn): do **not** try to “undo” the sibling-repo change. Surface the violation
in the answer, include the sibling repo SHA(s) and a precise diff summary, and
write a retroactive `docs/HANDOFF_<REPO>_<TOPIC>.md` so the change is at least
captured as a tracked handoff rather than an invisible side-effect.

## 4. Execution Rule
Active work executes through:
1. `workset -> epic -> bead`
2. `docs/WORKSET_REGISTER.md`
3. `.beads/`

Do not edit `.beads/` files directly.
Use `br` directly for bead mutations and inspection.

## 5. Change Rule
1. Keep `DnaOneCalc` narrower than `OxCalc`.
2. Preserve replay and comparison as first-class product surfaces.
3. Prefer implementation code, test code, and narrow spec or seam corrections over
   local documentation rollout.
4. If a change creates upstream seam pressure, capture it explicitly instead of
   silently normalizing local divergence.

## 6. Public Attribution
For any external or public-facing message authored by an agent, the first line must be:

*Posted by Codex agent on behalf of @govert*

This applies to outward-facing authored content such as handoffs, prompts, public
notes, or other messages intended for human readers outside the immediate repo
execution flow.

It does not apply to internal engineering artifacts such as git commit messages,
branch names, local bead updates, or other repo-internal tool metadata unless a
separate instruction explicitly says otherwise.
