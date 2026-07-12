# DNA OneCalc - Clarity & Terminology Improvements

## Key Changes Based on Feedback

### 1. **Consistent Terminology: "Formula Space" Everywhere**

**Before:**
- "New Formula" button + "Active Spaces" list (confusing!)
- "Workspace" section (what does that mean?)
- Mixed use of "Formula" and "Space"

**After:**
- ✅ "New Formula Space" button
- ✅ "Formula Spaces" list section
- ✅ "Quick Access" section (clearer than "Workspace")
- ✅ Consistent use of "Formula Space" throughout UI and code

**Mental Model:** A **Formula Space** is a single unit of work containing one formula with its configuration, evaluation context, and results. You work with multiple Formula Spaces, switching between them via the left rail and tab bar.

---

### 2. **More Accent Colors Throughout**

**Before:** Accent colors (terracotta, amber brass, moss) used sparingly, mostly for small badges.

**After:** Rich accent colors used liberally throughout:

**Quick Access Icons:**
- Home: Oxidized teal (#1e4d4a)
- Recent: Amber brass (#c88d2e)
- Pinned: Terracotta (#b84532)

**Result Metadata Grid (2x2):**
- Effective Display: Espresso ink (#1f1c17)
- Type: Amber brass (#c88d2e) ← now colored!
- Shape: Moss (#3e5238) ← now colored!
- Format: Terracotta (#b84532) ← now colored!

**Function Help Arguments:**
- name1: Oxidized teal (#1e4d4a)
- value1: Amber brass (#c88d2e)
- calc: Terracotta (#b84532)

**Left Rail Status Box:**
- Gradient background using oxidized teal → moss
- Colored labels for Profile/Floor (moss)

**Tab Bars:**
- Active Formula Space: Left border oxidized teal
- Evidence Workbench tab: Uses terracotta for active state

**Result:** The UI feels richer, warmer, and more distinctive without feeling overwhelming.

---

### 3. **Formula Walk Integrated into Formula Explorer**

**Before:** Formula Walk was hidden behind "Semantic X-Ray" mode tab, requiring navigation away from the editor.

**After:** Formula Walk is **always visible** in the right column of Formula Explorer, alongside Function Help.

**Layout:**
```
┌─────────────────────┬──────────────┐
│ Formula Editor      │ Formula Walk │
│                     │              │
├─────────────────────┤ (always      │
│ Result Display      │  visible)    │
│                     ├──────────────┤
│                     │ Function     │
│                     │ Help         │
└─────────────────────┴──────────────┘
```

**Benefits:**
- ✅ See evaluation structure while editing
- ✅ No mode switching required
- ✅ Natural workflow: edit → see walk → adjust
- ✅ Link to "Full Inspector →" for deep dive

---

### 4. **Simplified Navigation Model**

**Before:** Three confusing modes:
- "Formula Explorer" (mode)
- "Semantic X-Ray" (mode? view?)
- X-Ray drawer (also called "X-Ray Inspector")

**After:** Two clear views + optional inspector:

**Two Main Views:**
1. **Formula Explorer** (oxidized teal active state)
   - Formula editor
   - Result display
   - Formula Walk (integrated, always visible)
   - Function Help

2. **Evidence Workbench** (terracotta active state)
   - Twin Oracle comparison
   - Evidence lineage
   - Bundle management

**Optional Inspector Drawer:**
- **X-Ray Inspector** (right-side drawer)
- Opens when you click "Full Inspector →" or "Evaluate"
- Contains detailed metrics (Parse, Bind, Eval, Provenance)
- Also shows Formula Walk in larger format
- Can be closed without losing any functionality

**Navigation Mental Model:**
- **Views** = Complete workflows (tabs at top)
- **Inspector** = Optional detail dive (drawer from right)
- **Formula Spaces** = Work units (left rail + tab bar)

No confusion between modes, tabs, and drawers!

---

### 5. **Clearer Tab Bar Design**

**Active Formula Space:**
- White background
- **4px left border** in oxidized teal (was 2px, now bolder)
- Shadow for elevation
- Dirty indicator dot (amber)

**Inactive Formula Spaces:**
- Transparent background
- 4px transparent left border (maintains alignment)
- Hover state with parchment background

**Result:** Immediately clear which Formula Space is active.

---

### 6. **Clearer View Switcher**

**Before:** All tabs looked similar, unclear what the third "X-Ray" tab did.

**After:**
- **Formula Explorer** – Oxidized teal when active
- **Evidence Workbench** – Terracotta when active (different color!)
- Gradient background from parchment to warm tones
- No "X-Ray" mode tab (it's now a drawer)

**Result:** Two distinct workflows with visual identity, not three overlapping concepts.

---

## Visual Improvements

### More Gradients
- **Left rail status box:** Gradient from oxidized teal → moss
- **View switcher bar:** Gradient from parchment → warm tones
- **Help panel:** Gradient from amber brass (5% → 10%)
- **Formula Walk border:** Uses moss with 2px border

### More Border Weight
- **Active tab:** 4px left border (was 2px)
- **Formula Walk card:** 2px border in moss color
- **Help panel:** 2px border in amber brass color
- **Result metadata:** Each card has subtle border

### More Icon Color
- **Quick Access icons** use accent colors (not all gray)
- **Section heading icons** use accent colors
- **Status icons** in footer use subtle accent colors

---

## Information Architecture Clarity

### Left Rail Structure
```
┌─────────────────────────┐
│ [New Formula Space]     │ ← Clear action
├─────────────────────────┤
│ Quick Access            │ ← Navigation shortcuts
│   • Overview            │
│   • Recent              │
│   • Pinned              │
├─────────────────────────┤
│ Formula Spaces          │ ← Your work units
│   • LET Formula... (●)  │
│   • FILTER Examples     │
│   • Array Operations    │
├─────────────────────────┤
│ [Status: All Modes...] │ ← System status
└─────────────────────────┘
```

### Main Content Structure
```
┌─────────────────────────────────────────┐
│ [Tab Bar] Active Formula Spaces         │
├─────────────────────────────────────────┤
│ [View Switcher] Explorer | Workbench    │
├─────────────────────────────────────────┤
│                                         │
│ View Content (Explorer or Workbench)   │
│                                         │
└─────────────────────────────────────────┘
```

### Formula Explorer Layout
```
┌──────────────────────┬────────────┐
│ Formula Editor       │ Formula    │
│                      │ Walk       │
│                      │ (integrated│
│                      │  always    │
├──────────────────────┤  visible)  │
│ Result Display       │            │
│ (with 2x2 colored    ├────────────┤
│  metadata grid)      │ Function   │
│                      │ Help       │
└──────────────────────┴────────────┘
```

### Optional Inspector (Drawer)
```
┌──────────────┬─────────────────┐
│              │ X-Ray Inspector │
│  Main View   │ ┌─────────────┐ │
│              │ │Overview|Walk│ │
│              │ └─────────────┘ │
│              │                 │
│              │ Parse Context   │
│              │ Bind Context    │
│              │ Eval Context    │
│              │ Provenance      │
└──────────────┴─────────────────┘
```

---

## Terminology Decisions

### ✅ Use "Formula Space"
A complete work unit containing:
- The formula text
- Evaluation context
- Result
- Configuration
- State (dirty, pinned, etc.)

### ✅ Use "Quick Access"
Navigation shortcuts to views across all Formula Spaces:
- Overview (all spaces)
- Recent (recently used spaces)
- Pinned (favorited spaces)

### ✅ Use "Formula Explorer"
The primary workflow view where you:
- Edit formulas
- See results
- Walk evaluation structure
- Get function help

### ✅ Use "Evidence Workbench"
The comparison/proof workflow where you:
- Compare DNA vs Excel results
- Inspect evidence lineage
- Manage evidence bundles
- Export handoff packets

### ✅ Use "X-Ray Inspector"
Optional detailed inspection drawer showing:
- Parse/Bind/Eval/Provenance metrics
- Formula Walk in expanded format
- Deep dive technical details

### ❌ Avoid "Workspace"
Too vague. Use "Quick Access" for navigation, "Formula Spaces" for work units.

### ❌ Avoid "Semantic X-Ray" as a mode
It's confusing. "X-Ray Inspector" is a drawer, not a separate view.

### ❌ Avoid mixing "Formula" and "Space"
Always use "Formula Space" as the complete term.

---

## User Mental Model

### What am I working on?
**Formula Spaces** – shown in left rail and tab bar. Each is a complete unit of work.

### How do I navigate?
**Quick Access** – jump to overview/recent/pinned  
**View Switcher** – toggle between Explorer and Workbench workflows

### Where do I do my work?
**Formula Explorer** – for editing and evaluation  
**Evidence Workbench** – for comparison and proof management

### How do I see details?
**Formula Walk** – always visible in Explorer (right column)  
**X-Ray Inspector** – optional drawer for deep technical metrics

---

## Color Identity by Section

| Section | Primary Color | Usage |
|---------|--------------|-------|
| **Formula Editor** | Oxidized Teal | Syntax highlighting (functions), active states |
| **Formula Walk** | Moss | Border, section identity, binding nodes |
| **Function Help** | Amber Brass | Border, section identity, value arguments |
| **Result Display** | Mixed | Teal for result value, colored metadata |
| **Evidence Workbench** | Terracotta | Tab active state, Excel badges |
| **X-Ray Inspector** | Oxidized Teal | Header icon, active tab |

Each major section has visual identity through **border color** and **icon color**, making the UI easier to scan.

---

## Implementation Benefits

### For Users
- ✅ Clear where to find things
- ✅ Consistent terminology
- ✅ Visual color cues for sections
- ✅ No confusing mode overlaps
- ✅ Formula Walk always accessible

### For Development
- ✅ Clear component boundaries
- ✅ Simple state model (2 views, 1 drawer)
- ✅ Consistent naming in code
- ✅ Easy to extend (add new Formula Spaces, new views)

### For Documentation
- ✅ Easy to explain: "Two views, optional inspector"
- ✅ Clear terminology for support
- ✅ No ambiguity in user guides

---

## Before/After Comparison

| Aspect | Before | After |
|--------|--------|-------|
| **Work Unit Term** | "Formula" + "Space" mixed | "Formula Space" consistently |
| **Left Rail Sections** | "Workspace" (vague) | "Quick Access" (clear) |
| **Formula Walk Location** | Hidden in "X-Ray mode" | Always visible in Explorer |
| **Navigation** | 3 modes (confusing) | 2 views + drawer (clear) |
| **Active Tab Indicator** | 2px subtle border | 4px bold left border |
| **Accent Color Usage** | Minimal, mostly badges | Throughout (icons, metadata, borders) |
| **View Identity** | All tabs same color | Explorer=teal, Workbench=terracotta |
| **Inspector** | Confused with mode | Clear optional drawer |

---

## Next Steps (Not Implemented)

These improvements maintain the current scope while suggesting future enhancements:

1. **Formula Space Management**
   - Drag to reorder
   - Right-click context menu
   - Bulk actions (close all, pin all recent)

2. **Quick Access Smart Views**
   - "Dirty Formulas" filter
   - "Blocked Evaluations" filter
   - "Recent Comparisons" in Workbench

3. **Formula Walk Enhancements**
   - Click node to highlight in editor
   - Filter by status (show only blocked)
   - Expand/collapse all controls

4. **Keyboard Shortcuts**
   - Cmd+1: Formula Explorer
   - Cmd+2: Evidence Workbench
   - Cmd+I: Toggle Inspector
   - Cmd+W: Close Formula Space

All maintain the clarified terminology and structure!
