# Command A+ Technical Card Layout

Status: Style reference only.

Source: image attached in Codex conversation on 2026-05-20. The app did not expose the attached bitmap as a local file, so this note preserves the design cues for later prototype work.

## Why Save It

This is a useful counterpoint to the current warm, utilitarian TreeCalc mockups: it keeps a serious technical-diagram structure, but adds more personality through typography, color accents, and varied layout rhythm. It may be worth considering for a presentation/read-only skin, model-inspection views, template explainer panels, or high-density documentation-like UI surfaces.

## Visual Cues

- Large editorial serif title paired with compact uppercase sans labels.
- Mixed typography: serif for headings/formulas/emphasis, sans for UI labels, monospace for parameters and code-like values.
- Off-white paper background with subtle grey dividers and low-contrast panel borders.
- Muted accent palette: teal/green, rust/orange, ochre, grey, and black text rather than one dominant brand hue.
- Dense metric strip across the top with large numbers, small unit suffixes, and uppercase captions.
- Rounded technical cards with section bullets, horizontal rules, equations, notes, and mini bar visuals.
- Diagram flow lines and arrows connecting blocks, giving the layout a calculation-graph feel.
- Comfortable asymmetry: one wide top block, two lower blocks with different content weights, residual/feedback loop annotations.
- Inline callouts that feel integrated rather than modal: small colored badges, equation-side notes, and compact explanatory prose.

## Possible TreeCalc Uses

- A richer **read-only / presentation skin** for explaining a finished model.
- A **template anatomy** view showing parameters, formulas, inherited formatting, and instance behavior.
- A **dependency inspection** skin where formula blocks, value summaries, and graph arrows share one page.
- A **model card** export style for reports: compact facts at top, key formulas below, dependency/residual flow in the body.

## Cautions

- This should not become the default editing surface without testing. The mixed-font, editorial feel is engaging, but could reduce speed and predictability for day-to-day formula editing.
- Use the palette as accents, not as a full theme replacement.
- Keep the dense typography for inspection/presentation contexts; editable controls still need standard, predictable UI affordances.
