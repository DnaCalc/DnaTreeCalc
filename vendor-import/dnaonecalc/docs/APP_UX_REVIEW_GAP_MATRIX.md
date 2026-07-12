# APP_UX_REVIEW_GAP_MATRIX

This review artifact is the redesign source of truth for the current OneCalc UI pass.

Authorities used:
1. `docs/APP_UX_BRIEF.md`
2. `docs/APP_UX_ARCHITECTURE.md`
3. `docs/APP_UX_USE_CASES.md`
4. `docs/APP_UX_SCREEN_SPEC_EXPLORE.md`
5. `docs/APP_UX_SCREEN_SPEC_INSPECT.md`
6. `docs/APP_UX_SCREEN_SPEC_WORKBENCH.md`
7. `docs/APP_UX_PANEL_INVENTORY.md`
8. local Figma direction under `docs/ux_artifacts/figma_make/2026-04-04/*`

Scoring:
- `good`
- `partial`
- `weak`

Severity:
- `critical`
- `important`
- `polish`

## Explore
| Dimension | Score | Severity | Keep | Fix | Remove | Add |
| --- | --- | --- | --- | --- | --- | --- |
| information hierarchy fidelity | partial | critical | live formula editing, result, assist all exist | make editor/result/help read as one composition instead of fragmented cards | duplicated host/truth/status fragments inside editor body | stronger center result hero and calmer right assist panel |
| persona/use-case coverage | partial | important | long-form authoring, diagnostics, completion, signature help, array preview exist | make EX unexpected result, invalid repair, formatting truth check, and discovery flows easier to read in one glance | status chips that compete with authoring | result/effective-display distinction and scenario-policy summary |
| object ownership clarity | weak | critical | editor owns text/diagnostics/assist interaction | move host/profile/capability/mode truth to shell header/footer | repeated shell context in panel cards | explicit shell-owned context and footer status |
| primary vs secondary visibility | weak | critical | editor is primary in code | make result clearly secondary-primary and assist clearly secondary-supporting | equal-weight card noise | cleaner three-column ranking |
| workflow continuity across modes | partial | important | inspect/workbench handoff language exists | make Explore clearly feel like the start of a mode sequence | duplicated X-Ray promotion copy | clearer next-step affordance into Inspect |
| honesty surfaces for capability/blocked/lossy truth | partial | important | truth source, blocked reason, capability floor exist | centralize and simplify them | panel-local truth repetition | footer/runtime truth strip |
| visual clarity and comfort | partial | important | warm editorial direction is present | increase spacing discipline and reduce abrupt card boundaries | noisy meta strips | calmer premium editor/result/help rhythm |
| keyboard-first viability | partial | important | live input, tab handling, completion commands exist | keep authoring details visually close to keyboard focus | none | visible focus ownership for assist/help |
| browser-responsive credibility | partial | polish | current grid is usable | make the three-cluster layout degrade cleanly | overly rigid two-column assumptions | responsive cluster stacking |

## Inspect
| Dimension | Score | Severity | Keep | Fix | Remove | Add |
| --- | --- | --- | --- | --- | --- | --- |
| information hierarchy fidelity | partial | critical | formula walk, summary cards, retained context exist | make walk clearly dominant and summary subordinate | equal-weight summary and evidence cards | stronger overview + x-ray main column |
| persona/use-case coverage | partial | important | parse/bind/eval/provenance surfaces exist | align better with IN deep-tree, blocked reason, and Excel cross-check flows | excess summary duplication | node detail drawer-ready framing |
| object ownership clarity | partial | important | inspect mode owns semantic evidence | reduce shell-like context repeated inside mode cards | repeated host/profile restatement | tighter inspect summary stack |
| primary vs secondary visibility | weak | critical | formula walk is present | make it obviously primary | dense secondary cards beside it | source/result anchor column |
| workflow continuity across modes | partial | important | retained artifact can open into Inspect | make retained comparison context secondary, not the screen’s main story | over-prominent retained summary blocks | stronger “from Explore” and “from Workbench” orientation |
| honesty surfaces for capability/blocked/lossy truth | partial | important | blocked reasons and provenance are present | surface them with clearer semantic labels | flattened evidence labels | distinct capability/provenance section |
| visual clarity and comfort | partial | important | premium card treatment exists | reduce dump-like feel | generic nested card rhythm | stronger X-Ray hierarchy |
| keyboard-first viability | partial | polish | readable in browser | improve focus order later | none | future node-detail drawer focus path |
| browser-responsive credibility | partial | polish | current layout works | rebalance walk/summary proportions | none | responsive dominant walk column |

## Workbench
| Dimension | Score | Severity | Keep | Fix | Remove | Add |
| --- | --- | --- | --- | --- | --- | --- |
| information hierarchy fidelity | partial | critical | outcome, evidence, lineage, actions, catalog exist | make outcome/reliability/next action immediately obvious | long mixed evidence flow | stronger triage-first ordering |
| persona/use-case coverage | partial | important | retained import/open, discrepancy summary, explain records exist | align better with WB decision, escalation, and blocked-dimension reading | utility-form feel in import surfaces | clearer recommended-action and reliability surfaces |
| object ownership clarity | partial | important | workbench owns retained artifact and replay evidence | move shell context out of outcome cards | repeated context tags | clearer evidence-envelope vs action sections |
| primary vs secondary visibility | weak | critical | outcome hero exists | make lineage/evidence/actions visually ranked | catalog competing with main triage surface | main outcome board plus secondary evidence grid |
| workflow continuity across modes | partial | important | open in Workbench / Inspect exists | strengthen transition from imported artifact to semantic dissection | catalog-first reading order | explicit escalation and inspect action path |
| honesty surfaces for capability/blocked/lossy truth | partial | critical | projection coverage gaps and display divergence are preserved | distinguish blocked dimension, display divergence, and semantic mismatch more clearly | generic mismatch grouping | reliability/coverage treatment |
| visual clarity and comfort | partial | important | warm editorial direction exists | reduce dense mixed-card feeling | crowded evidence stacking | calmer outcome + evidence boards |
| keyboard-first viability | partial | polish | browser interaction works | improve focus path later | none | future action-order refinement |
| browser-responsive credibility | partial | polish | current layout renders | improve dominance and stack order on smaller widths | none | responsive triage-first stacking |

## Shell-Wide Findings
| Area | Severity | Keep | Fix | Remove | Add |
| --- | --- | --- | --- | --- | --- |
| left rail | important | formula-space list and active-space identity | treat it as workspace navigation only | repeated deep runtime facts | cleaner pinned/open identity |
| top context bar | critical | mode switch and active space title | make it the owner of active scenario, truth source, host, packet, capability summary | mode-local context duplication | compact context facts |
| main canvas | critical | current mode routing | keep only mode-primary and nearby secondary surfaces in the canvas | cross-mode context duplication | stricter composition per mode |
| right drawer | important | none yet | introduce drawer-ready secondary/reference ownership | ad-hoc secondary detail in body cards | settings/details drawer path |
| footer | critical | none yet | add runtime/session/capability/build truth ownership | panel-local runtime repetition | compact footer status strip |

## Ranked Findings

### Critical
1. `Explore` still duplicates shell-owned truth inside the editor surface.
2. `Explore` does not yet read as editor + result + help with clear primary/secondary/supporting ranks.
3. `Inspect` formula walk is not yet visually dominant enough to deliver the intended X-Ray reading path.
4. `Workbench` does not yet make outcome, reliability, and next action the first readable story.
5. The shell lacks a real footer owner for runtime and capability truth.

### Important
1. Mode bodies still restate host/profile/capability truth that should live in shell chrome.
2. `Explore` result/effective-display distinction is present but not strong enough.
3. `Workbench` needs clearer blocked-dimension versus mismatch semantics in the visual hierarchy.
4. `Inspect` retained comparison context should support, not dominate, the semantic inspection story.

### Polish
1. Responsive degradation can be calmer and more intentional across all three modes.
2. Keyboard ownership cues should become more visible in assist-heavy surfaces.
3. The visual system is warm enough, but still needs calmer density and stronger compositional rhythm.
