# FLOW — the model that thinks out loud

A keyboard-first, modeless calc skin for DNA TreeCalc that lays a focused slice of the model out as a
left-to-right **sentence** in dependency order, and — when you press **F9** — sweeps a reading-head
through it in the engine's real evaluation order, rolling each changed value to its new number. It is
Excel's F9 + Trace Precedents + Name Box + Evaluate Formula, fused into one alive surface that shows
the one thing no grid ever has: **your model in the act of computing.**

> *Arrow to a number, press F9, and watch your model think — laid out in dependency order, animated in
> the exact order the engine actually solved it.*

## This folder

- **[`FLOW_DESIGN.md`](FLOW_DESIGN.md)** — the concrete design, in two tiers: **what ships on today's
  Skin IR** (the entire first-touch wow) and **what the stack requirements deepen**. Surfaces,
  full keyboard map, magic moments, visual language, `FlowSkinState`, zippiness, honest limitations.
- **[`FLOW_VISION.md`](FLOW_VISION.md)** — the expansive re-think *assuming the
  [stack requirements](../stack-requirements/) are in place*: where FLOW goes from "watch the model
  think" to a full **modeling instrument** — explain, simulate, time-travel, compare, and explore. The
  "something special" capture for the next design + implementation stages.
- **[`reference/FLOW_TOURNAMENT.raw.json`](reference/FLOW_TOURNAMENT.raw.json)** — the raw 6-concept
  tournament + judge panel + adversarial critique synthesis, verbatim, for provenance.

## See also

- The openable, animated mockup: [`../prototypes/09_flow.html`](../prototypes/09_flow.html).
- The capabilities FLOW asks the stack for: [`../stack-requirements/`](../stack-requirements/) — FLOW
  is the primary consumer that motivates that requirement set.
- The skin architecture this builds on: [`../SKINS.md`](../SKINS.md).

## The spine in one breath

Modeless (the only mode is the 1-bit *selected* vs *editing* distinction every spreadsheet has). The
**layout axis** is a stable host-side topological sort (always populated, never reshuffles); the
**animation axis** is the engine's `last_run.evaluation_order` (used for the F9 sweep only). Every
visible thing is a read over the published `WorkspaceState` or one of the existing intents — **nothing
invents engine-owned truth.** The first-touch wow ships on today's IR; three reviewed intent additions
(`SelectNodes`, `ReplicateContent`, `PreviewEdit`) turn it from *watch the model think* into *bend it*.
