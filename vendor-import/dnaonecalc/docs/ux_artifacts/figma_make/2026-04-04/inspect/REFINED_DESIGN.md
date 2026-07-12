# DNA OneCalc - Refined Warm Editorial Design

## Overview
This refined design takes the best elements from the original Warm Editorial direction and enhances them with richer colors inspired by the Modular Evidence Cockpit, while maintaining a light mode experience that feels premium, intentional, and visually confident.

## Design Philosophy

### Not Conservative Enterprise
This design actively avoids:
- Generic beige/gray corporate palettes
- Purple-on-white SaaS defaults
- Cold, sterile dashboard patterns
- Timid, apologetic color application

### Bold, Warm, Editorial
Instead, it embraces:
- **Oxidized teal** (#1e4d4a) - Deep, serious primary actions
- **Terracotta** (#b84532) - Warm, grounded comparison states
- **Amber brass** (#c88d2e) - Rich, confident highlights
- **Moss** (#3e5238) - Natural, calming secondary tones
- **Espresso ink** (#1f1c17) - Sophisticated dark text
- **Parchment** (#f7f3ea) - Warm, inviting surfaces
- **Warm smoke** (#7a7568) - Elegant muted text

This palette creates a distinctive, memorable experience that feels both technical and approachable.

---

## Three Refined Areas

### 1. Formula Explorer - Premium Editor Experience

#### Enhanced Formula Editor
**Previous**: Basic syntax highlighting in standard container  
**Refined**: Premium multi-layer editor with:
- **Bordered editor surface** with 2px border for visual weight
- **Integrated toolbar** showing formula name, line count, profile
- **Line numbers** in warm smoke with subtle divider
- **Advanced syntax highlighting** via positioned overlay
  - Functions: Oxidized teal, bold
  - Numbers: Amber brass
  - Operators: Terracotta
  - Default text: Espresso ink
- **Visual status indicators** with color-coded badges
- **Elevated surface treatment** with subtle shadow

#### Improved Result Presentation
**Previous**: Basic result card  
**Refined**: 
- **Gradient background** (oxidized teal 5% → 10%) with 2px colored border
- **Large, confident typography** (48px mono for the result value)
- **Detailed metadata grid** with 4-column breakdown
- **Visual hierarchy** through size, weight, and color

#### Better Integration of Help
**Previous**: Help surface separate from workflow  
**Refined**:
- **Side-by-side layout** with result (50/50 split)
- **Argument breakdown** with inline descriptions
- **Status badges** with visual indicators
- **Call-to-action** to open X-Ray inspector
- **Consistent surface treatment** matching editor quality

Result: The editor feels like a **first-class product surface**, not an improvised textarea.

---

### 2. X-Ray Inspector - Two-Level Semantic Inspection

#### Two-Tab System
**Overview Tab**: High-level semantic summary
- Parse Context
- Bind Context  
- Eval Context
- Provenance

**Formula Walk Tab**: Detailed tree-based evaluation

#### Formula Walk - The Star Feature
A structured, collapsible tree view showing:

**Node Types**:
- **Function** (oxidized teal) - LET, SEQUENCE, FILTER, SUM, MOD
- **Binding** (moss) - Named variables like `values`, `filtered`
- **Reference** (terracotta) - Variable references
- **Value** (amber brass) - Literal values

**Status Badges**:
- **Evaluated** (green check) - Function produced a result
- **Bound** (amber dot) - Variable is bound to a value
- **Opaque** (gray minus) - Details unavailable
- **Blocked** (red minus) - Evaluation blocked by capability

**Tree Interaction**:
- **Collapsible nodes** with chevron indicators
- **Hover badges** showing status on hover
- **Inline values** showing intermediate results
- **Depth indentation** (20px per level)
- **Color-coded by type** for quick scanning

**Example Tree**:
```
✓ LET → 30
  ├─ ● values
  │   └─ ✓ SEQUENCE(10, 1, 1, 1) → [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
  ├─ ● filtered
  │   └─ ✓ FILTER → [2, 4, 6, 8, 10]
  │       ├─ ● values → [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
  │       └─ ✓ MOD(values, 2) = 0 → predicate
  └─ ✓ SUM(filtered) → 30
      └─ ● filtered → [2, 4, 6, 8, 10]
```

**Legend Grid**: 2x2 grid explaining status icons

This transforms X-Ray from a "developer dump" into a **serious semantic inspection surface** that shows the journey from formula structure to final result.

---

### 3. Evidence Workbench - Stronger Lineage & Session Truth

#### Enhanced Comparison Header
**Previous**: Basic comparison badge  
**Refined**:
- **Gradient hero section** with rich background
- **Large Match badge** with icon
- **4-column metadata grid**: Reliability, Envelope, Platform, Timestamp
- **Visual hierarchy** through size and placement

#### Evidence Lineage Box
**Previous**: Simple list  
**Refined**: Structured, indented hierarchy showing:

```
→ Scenario: LET Formula Analysis
  scn_20260403_247 • Created 2026-04-03 14:30
  
  → DNA Run #247
    run_247_001 • OxFml v0.12.4 • OxFunc v0.8.2
    [30]
  
  → Excel Observation
    obs_247_xl365 • OxXlPlay v0.4.1 • Excel 365 • Windows 11
    [30]
  
    🧪 Comparison Generated
      cmp_247_001 • Full envelope • High reliability
```

**Features**:
- **Chevron-based indentation** showing parent-child relationships
- **Colored result badges** (DNA = oxidized teal, Excel = terracotta)
- **Full lineage IDs** with descriptive metadata
- **Comparison highlight** with beaker icon and accent background

#### Evidence Bundle Controls
**Previous**: Generic action buttons  
**Refined**:
- **3-column ID grid** showing Scenario, Run, Comparison IDs
- **Prominent action buttons** with icons
- **Primary/secondary hierarchy** (Retain vs Export)
- **Visual weight** through shadows and borders

#### Status Footer - Session Truth
**New feature** showing meaningful operational context:

**Left Side**:
- **Ready indicator** with green dot
- **Host profile** with CPU icon: H1-Standard
- **Engine versions** with package icons: OxFml v0.12.4, OxFunc v0.8.2

**Right Side**:
- **Performance metric** with zap icon: 1.2ms
- **Product version**: DNA OneCalc v0.1.0

This creates a **serious proving host** feel, not a toy calculator.

---

## Color Application Strategy

### Primary Actions & Success
- **Oxidized Teal** (#1e4d4a, #2d6864)
- Used for: Primary buttons, active states, success badges, matches
- Why: Deep, trustworthy, technical but warm

### Warnings & Comparison States  
- **Terracotta** (#b84532, #d15745)
- Used for: Excel observation badges, operators in code, comparison differentiators
- Why: Warm but serious, draws attention without alarm

### Highlights & Values
- **Amber Brass** (#c88d2e, #dda947)
- Used for: Numbers in code, evaluation badges, accent highlights
- Why: Rich, valuable, distinctive

### Supporting Elements
- **Moss** (#3e5238, #566b4f)
- Used for: Binding nodes, secondary icons, supporting badges
- Why: Natural, calming, complements without competing

### Text Hierarchy
- **Espresso Ink** (#1f1c17) - Primary text, headings
- **Warm Smoke** (#7a7568) - Secondary text, metadata
- Why: High contrast without harshness, warm without weakness

### Surfaces
- **Parchment** (#f7f3ea, #ede7da) - Cards, sidebar, surfaces
- **Background** (#faf7f1) - Main canvas
- Why: Warm, inviting, editorial feel without being retro

---

## Visual Refinements

### Borders & Depth
- **2px borders** on important surfaces (editor, result, cards)
- **Opacity-based borders** (0.1, 0.15, 0.2) for hierarchy
- **Subtle shadows** on interactive elements
- **Gradient backgrounds** for emphasis without noise

### Typography
- **Espresso ink** for primary text (not pure black)
- **15px mono** for code (larger than typical 13px)
- **Semibold weights** for headings (not just medium)
- **7px leading** on code for breathing room

### Interactive States
- **Hover backgrounds** with parchment tones
- **Transition-all** for smooth state changes
- **Scale transforms** (1.01, 1.02) for depth
- **Color shifts** on hover (oxidized teal → lighter variant)

### Spacing
- **6px (1.5rem) gaps** between major sections
- **4px (1rem) gaps** within sections
- **Consistent padding**: 16px (cards), 24px (containers)
- **Deliberate whitespace** preventing cramped feel

---

## Browser Responsiveness

While desktop-first, the design degrades gracefully:

### X-Ray Drawer
- **420px fixed width** on desktop
- **Collapses completely** on narrow screens
- **Content reflows** to full width when drawer closes

### Grid Layouts
- **2-column grids** (result + help) stack to 1 column
- **3-column** and **4-column** metadata grids adapt
- **Tab bar** shows fewer tabs, overflow scrolls

### Priority Preservation
1. Formula editor - always visible, full quality
2. Result display - always visible
3. Help/diagnostics - stacks below on narrow screens
4. X-Ray drawer - hides first
5. Left rail - can collapse to icons

---

## Implementation Notes

### Component Architecture
- **PremiumFormulaEditor**: Self-contained editor with syntax highlighting
- **FormulaWalkInspector**: Recursive tree component with status badges
- **RefinedWarmEditorial**: Main shell with three view modes

### State Management
- View mode: `explorer` | `xray` | `workbench`
- X-Ray tab: `overview` | `walk`
- Drawer state: `open` | `closed`

### Performance
- **Syntax highlighting** via positioned overlay (no contentEditable complexity)
- **Tree expansion** state managed per node
- **Transitions** use GPU-accelerated properties (transform, opacity)

---

## Success Criteria

This refined design succeeds when:

1. ✅ The formula editor feels **premium and intentional**, not improvised
2. ✅ The color palette is **distinctive and memorable**, not conservative
3. ✅ X-Ray shows **semantic structure**, not just a dump
4. ✅ Formula Walk reveals **evaluation lineage** clearly
5. ✅ Evidence workbench feels like a **serious proving host**
6. ✅ The status footer provides **meaningful session truth**
7. ✅ The design remains **highly readable** in light mode
8. ✅ Supporting surfaces stay **subordinate** to the main task
9. ✅ The product feels **warmer, bolder, more confident** than generic tools
10. ✅ Browser responsiveness is **strong** without sacrificing desktop quality

---

## Comparison to Original Directions

| Aspect | Original Warm Editorial | Refined |
|--------|------------------------|---------|
| **Palette** | Softer teal, lighter amber | Oxidized teal, amber brass - richer |
| **Editor** | Basic container | Premium bordered surface with toolbar |
| **X-Ray** | Single overview | Two levels: Overview + Formula Walk |
| **Tree View** | None | Full collapsible evaluation tree |
| **Status Footer** | None | Comprehensive session truth |
| **Evidence** | Basic lineage | Indented hierarchy with visual badges |
| **Borders** | 1px subtle | 2px intentional on key surfaces |
| **Typography** | Standard weights | Bolder, more confident |
| **Feel** | Warm, approachable | Warm, **bold**, premium |

---

## Future Enhancements

Not in current scope but compatible with this direction:

1. **Split pane views** for side-by-side comparison
2. **Formula Walk filtering** (show only evaluated, hide bound, etc.)
3. **Interactive tree nodes** to jump to formula positions
4. **Performance profiling** in X-Ray with flame graphs
5. **Evidence bundle preview** before export
6. **Capability diff viewer** comparing two snapshots
7. **Handoff packet templating** for common upstream actions
8. **Secondary windows** for advanced monitoring

All would use the same refined palette and visual language.
