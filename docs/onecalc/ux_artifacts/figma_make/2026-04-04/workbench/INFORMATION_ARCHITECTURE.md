# DNA OneCalc - Information Architecture

## Overview

DNA OneCalc uses a **single-shell, three-mode architecture** where each formula space operates in one of three explicit task modes. This document defines the ownership model, shell structure, and mode-specific hierarchies.

---

## Ownership Model (Four Levels)

### 1. Workspace Level
**Persists across all formula spaces**

**Where it lives:** Left rail footer, workspace navigation

**Contains:**
- Workspace navigation (overview, recent, pinned)
- Open/recent/pinned formula spaces list
- **Host profile summary** (OC-H0, platform, runtime)
- **Capability center** (function count, capability floor, supported functions)
- **Extension management** (OxXlPlay status, extension list)
- **Platform gates** (browser blocked, native admitted)
- **Environment truth** (runtime class, seam pin set)

**User actions:**
- Navigate between formula spaces
- Create new formula space
- Access workspace-level views
- Manage extensions
- View capability summary

---

### 2. Formula-Space Level
**Specific to one formula space (the active unit of work)**

**Where it lives:** Main canvas, context bar, right drawer

**Contains:**
- **Formula text** (the formula being edited)
- **Editor state** (cursor position, selection, undo/redo)
- **Completions list** (available functions based on capability)
- **Current help** (documentation for active function)
- **Diagnostics** (parse errors, validation messages)
- **Result** (evaluated output)
- **Effective display** (formatting, precision, style)
- **Scenario policy controls** (deterministic vs real-time/rand)
- **Formatting editor** (number format, decimals)
- **Conditional formatting editor** (rules, color scales, data bars)
- **Inspect/X-Ray state** (parse/bind/eval summaries)
- **Retained runs** (list of saved run results)
- **Compare state** (active comparisons)

**User actions:**
- Edit formula
- Evaluate formula
- View/edit formatting
- Set scenario policies
- Switch between modes
- Access completions and help
- View diagnostics

---

### 3. Run Level
**Represents a single evaluation of a formula**

**Where it lives:** Inspect mode summaries, Workbench mode lineage

**Contains:**
- **Run metadata** (run ID, timestamp, scenario ID)
- **Packet kind** (formula_edit, edit_accept_recalc, replay_capture)
- **Timing** (duration in ms)
- **Replay capture** (snapshot for replay)
- **Lineage entry** (parent scenario, engine versions)

**User actions:**
- View run details
- Retain run for comparison
- Inspect packet context
- Review timing data

---

### 4. Comparison Level
**Represents a comparison between DNA OneCalc and Excel results**

**Where it lives:** Workbench mode dominant canvas

**Contains:**
- **Comparison envelope** (full, partial, blocked dimensions)
- **Reliability** (high, medium, low confidence)
- **Mismatches** (value differences, type mismatches)
- **Blocked dimensions** (unavailable comparisons due to capability limits)
- **Widening/handoff status** (admission eligibility, upstream ready)
- **Evidence bundle** (package for export)
- **Observation envelope** (Excel-side captured data)

**User actions:**
- Review comparison outcome
- Inspect envelope coverage
- Identify blocked dimensions
- Retain as evidence
- Export handoff packet
- Mark for review

---

## Shell Model

### Structure
```
┌────────────────────────────────────────────────────────┐
│  Global Top Bar                                        │
│  (DNA OneCalc branding, search, help, settings)       │
├────────────┬───────────────────────────┬───────────────┤
│            │  Formula Space Context    │               │
│            │  (Active space title,     │               │
│  Left Rail │   mode switcher,          │  Right Drawer │
│            │   compact host truth)     │  (Secondary   │
│ (Workspace │                           │   details,    │
│  nav,      ├───────────────────────────┤   not primary │
│  spaces,   │                           │   navigation) │
│  env       │   Main Canvas             │               │
│  truth)    │   (Changes by mode)       │               │
│            │                           │               │
│            │                           │               │
├────────────┴───────────────────────────┴───────────────┤
│  Status Footer                                         │
│  (Ready state, mode, versions)                         │
└────────────────────────────────────────────────────────┘
```

### Left Rail (Workspace Level)
**Fixed width: 256px**

**Sections:**
1. **New Formula Space** button (always visible)
2. **Workspace navigation** (Overview, Recent, Pinned)
3. **Formula Spaces** (collapsible list of open spaces)
4. **Extensions** (collapsible extension manager)
5. **Environment Truth** (host profile, capability, platform gates)

**Interaction:**
- Click formula space to switch active space
- Drag to reorder spaces (future)
- Collapse sections to save space

---

### Top Context Bar (Formula-Space Level)
**Height: 48px**

**Layout:**
```
┌────────────────────────────────────────────────────────┐
│ [Formula Space Title + Dirty Dot] [Scenario Policy ▼] │
│                                                        │
│     [Explore] [Inspect] [Workbench]  [Host Truth]     │
└────────────────────────────────────────────────────────┘
```

**Left:** Formula space title, dirty indicator, scenario policy dropdown
**Center:** Mode switcher (Explore • Inspect • Workbench)
**Right:** Compact host truth (timing, OC-H0, function count)

**Color-coded mode buttons:**
- **Explore:** Oxidized teal (#1e4d4a)
- **Inspect:** Moss (#3e5238)
- **Workbench:** Terracotta (#b84532)

---

### Main Canvas (Mode-Specific)
**Changes dramatically by mode** - see Mode Hierarchy section below

---

### Right Drawer (Secondary Details)
**Width: 400px • Slides in from right**

**Purpose:** Show **secondary detail**, not primary navigation

**Types:**
- **Completions** (function list)
- **Help** (function documentation)
- **Formatting** (number format, conditional formatting editors)
- **Details** (full diagnostic dump, capability snapshot)
- **Flags** (all function flags and status)
- **Evidence** (evidence bundle contents)
- **Envelope** (full observation envelope)

**Interaction:**
- Opens via "Show completions →" or "Details →" links
- Closes with X button or Esc key
- Does NOT change mode
- One drawer at a time

---

### Status Footer
**Height: 32px**

**Left:** Ready indicator, engine versions (OxFml, OxFunc)
**Right:** Active mode, product version

---

## Mode Hierarchy

### Explore Mode (Formula Editing)
**Purpose:** Create, edit, and evaluate formulas with immediate result feedback

**Color:** Oxidized teal (#1e4d4a)

**Main Canvas Layout:**
```
┌─────────────────────────────────────────┐
│  Formula Editor (dominant, 2/3 width)   │
│  - Large editor with line numbers       │
│  - Syntax highlighting                  │
│  - Diagnostics inline                   │
│  - Evaluate button                      │
│                                         │
├─────────────────────┬───────────────────┤
│  Result Display     │  Array Preview    │
│  - Large value      │  - Intermediate   │
│  - Type/shape       │    arrays         │
│  - Effective display│  - Quick actions  │
└─────────────────────┴───────────────────┘
```

**Hierarchy (top to bottom):**
1. **Formula editor** - Dominant, largest surface
2. **Result display** - Large, prominent result value
3. **Effective display** - Formatting controls inline
4. **Diagnostics** - Errors/warnings if present
5. **Array preview** - Intermediate values

**Right Drawer Options:**
- **Completions** - Full function list (517 functions)
- **Help** - Current function documentation
- **Formatting** - Number format, conditional formatting editor

**Where things live:**
- ✅ **Deterministic vs realtime/rand policy:** Top context bar dropdown (scenario policy)
- ✅ **Formatting editor:** Right drawer (Formatting)
- ✅ **Conditional formatting editor:** Right drawer (Formatting)
- ✅ **Scenario-affecting host flags:** Right drawer (Formatting) or inline settings

---

### Inspect Mode (Semantic Analysis)
**Purpose:** Understand how the formula is parsed, bound, and evaluated

**Color:** Moss (#3e5238)

**Main Canvas Layout:**
```
┌───────────────────────────┬─────────────┐
│  Formula Walk (dominant)  │ Host State  │
│  - Collapsible tree       │ - Profile   │
│  - Evaluation nodes       │ - Platform  │
│  - Partial eval badges    │ - Runtime   │
│  - Color-coded by type    │             │
│                           │ Scenario    │
│ Parse Context             │ Flags       │
│ - Token count             │ - Checkboxes│
│ - Function count          │             │
│                           │ Packet      │
│ Bind Context              │ Context     │
│ - Variables bound         │ - Run token │
│ - Scope depth             │ - Timing    │
│                           │             │
│ Eval Context              │ Function    │
│ - Steps, duration         │ Flags       │
│                           │ - Status    │
└───────────────────────────┴─────────────┘
```

**Hierarchy (top to bottom):**
1. **Formula Walk** - Dominant, structural tree view
2. **Parse/Bind/Eval summaries** - Collapsible context sections
3. **Host state** - Capability, platform, runtime (right column)
4. **Scenario flags** - Editable checkboxes (right column)
5. **Packet context** - Run-level metadata (right column)
6. **Function flags** - Supported status per function (right column)

**Right Drawer Options:**
- **Details** - Full diagnostic dump, capability snapshot
- **Flags** - Complete function flag list (all 517 functions)

**Where things live:**
- ✅ **Scenario-affecting host flags:** Right column, editable checkboxes
- ✅ **Function flags:** Right column preview + right drawer full list
- ✅ **Capability summary:** Right column host state section

---

### Workbench Mode (Comparison & Evidence)
**Purpose:** Compare DNA vs Excel, manage evidence, export handoff packets

**Color:** Terracotta (#b84532)

**Main Canvas Layout:**
```
┌─────────────────────────────────┬─────────────┐
│  Comparison Outcome (dominant)  │ Reliability │
│  - Match/mismatch badge         │ - Quality   │
│  - Envelope summary             │ - Coverage  │
│                                 │             │
│  Replay Lineage                 │ Evidence    │
│  - Scenario → Run → Obs → Cmp  │ Bundle      │
│  - Indented hierarchy           │ - IDs       │
│  - Colored result badges        │ - Metadata  │
│                                 │             │
│  Observation Envelope           │ Handoff     │
│  - Coverage, dimensions         │ Actions     │
│  - Observed dims list           │ - Retain    │
│                                 │ - Export    │
│  Blocked Dimensions             │ - Review    │
│  - Warnings if any              │             │
│                                 │ Widening    │
│                                 │ Status      │
│                                 │             │
│                                 │ Retained    │
│                                 │ Runs        │
└─────────────────────────────────┴─────────────┘
```

**Hierarchy (top to bottom):**
1. **Comparison outcome** - Dominant, match/mismatch header
2. **Replay lineage** - Scenario → Run → Observation → Comparison
3. **Observation envelope** - What was captured
4. **Blocked dimensions** - Warnings/limitations
5. **Reliability** - Confidence metrics (right column)
6. **Evidence bundle** - Export package (right column)
7. **Handoff actions** - Retain, export, review (right column)
8. **Widening status** - Admission eligibility (right column)
9. **Retained runs** - Previous runs (right column)

**Right Drawer Options:**
- **Evidence** - Full evidence bundle contents
- **Envelope** - Complete observation envelope details

**Where things live:**
- ✅ **Evidence bundle:** Right column preview + right drawer full details
- ✅ **Reliability:** Right column, visual progress bars
- ✅ **Blocked dimensions:** Main canvas, warning panel
- ✅ **Handoff actions:** Right column, prominent buttons

---

## Surface Placement Map

| Surface | Level | Explore Mode | Inspect Mode | Workbench Mode |
|---------|-------|--------------|--------------|----------------|
| **Deterministic vs realtime/rand policy** | Formula-space | Context bar dropdown | Context bar dropdown | Context bar dropdown |
| **Scenario-affecting host flags** | Formula-space | Formatting drawer | Right column checkboxes | Not primary |
| **Formatting editor** | Formula-space | Right drawer | Not primary | Not primary |
| **Conditional formatting editor** | Formula-space | Right drawer | Not primary | Not primary |
| **Capability summary** | Workspace | Left rail footer | Right column | Left rail footer |
| **Extension state** | Workspace | Left rail | Left rail | Left rail |
| **Formula Walk** | Formula-space | Not primary | Dominant canvas | Not shown |
| **Parse/Bind/Eval summaries** | Formula-space | Not shown | Main canvas | Not shown |
| **Host state** | Formula-space | Not primary | Right column | Not primary |
| **Function flags** | Formula-space | Not shown | Right column + drawer | Not shown |
| **Packet context** | Run | Not shown | Right column | In lineage |
| **Comparison envelope** | Comparison | Not shown | Not shown | Main canvas |
| **Reliability** | Comparison | Not shown | Not shown | Right column |
| **Evidence bundle** | Comparison | Not shown | Not shown | Right column + drawer |
| **Handoff actions** | Comparison | Not shown | Not shown | Right column |

---

## Interaction Patterns

### Mode Switching
**User clicks:** Explore | Inspect | Workbench in context bar
**Result:** Main canvas **completely changes**, drawer closes
**State:** Formula-space retains all state (formula, result, retained runs)

### Drawer Opening
**User clicks:** "Show completions →", "Details →", "Edit →" links
**Result:** Right drawer slides in from right, main canvas shifts left
**Behavior:** One drawer at a time, Esc to close

### Scenario Policy
**User clicks:** Deterministic dropdown in context bar
**Result:** Dropdown shows three options (Deterministic, Real-time, Full Random)
**Effect:** Changes how NOW(), TODAY(), RAND() behave in evaluations

### Formula Space Switching
**User clicks:** Formula space in left rail
**Result:** Active space changes, context bar updates, mode persists
**State:** Each space remembers its last active mode

---

## Visual Identity by Mode

### Explore Mode
- **Color:** Oxidized teal (#1e4d4a)
- **Icon:** Pencil or Edit icon
- **Feel:** Focused, editor-first, results-oriented
- **Dominant:** Large formula editor, prominent result

### Inspect Mode
- **Color:** Moss (#3e5238)
- **Icon:** Tree or Layers icon
- **Feel:** Analytical, structural, technical
- **Dominant:** Formula Walk tree, semantic summaries

### Workbench Mode
- **Color:** Terracotta (#b84532)
- **Icon:** Compare or Package icon
- **Feel:** Evidence-based, comparison-focused, export-ready
- **Dominant:** Comparison outcome, replay lineage

---

## Progressive Disclosure Strategy

### Explore Mode
**Always visible:** Editor, result, diagnostics
**One click:** Completions, help, formatting
**Hidden by default:** Advanced formatting rules

### Inspect Mode
**Always visible:** Formula Walk
**Collapsed by default:** Parse/Bind/Eval summaries
**One click:** Full diagnostic dump, all function flags

### Workbench Mode
**Always visible:** Comparison outcome, lineage
**Collapsed by default:** Envelope details, blocked dimensions
**One click:** Evidence bundle contents, full envelope

---

## Responsive Behavior

### Desktop (1920x1080)
- Left rail: 256px
- Main canvas: Flexible
- Right drawer: 400px when open

### Narrow Desktop (1366x768)
- Left rail: 256px (collapsible to icons)
- Main canvas: Flexible
- Right drawer: 360px when open

### Tablet/Browser (1024x768)
- Left rail: Collapsible to icons (48px)
- Main canvas: Full width
- Right drawer: Overlays canvas (not side-by-side)

---

## Key Design Decisions

### Why Three Modes?
**Problem:** Mixing formula editing, semantic inspection, and comparison in one view creates cognitive overload.

**Solution:** Explicit modes with **completely different hierarchies** optimize for distinct tasks:
- **Explore** optimizes for editing and seeing results
- **Inspect** optimizes for understanding evaluation structure
- **Workbench** optimizes for proving correctness

### Why Not Tabs or Panels?
**Avoided:** Tabs imply similar content, panels imply simultaneous use.

**Chosen:** Mode switcher makes it clear that canvas **completely changes**, and you're doing a fundamentally different task.

### Why Right Drawer for Secondary Details?
**Avoided:** Making every detail primary (information overload).

**Chosen:** Right drawer keeps primary canvas clean, but details are **one click away**, not hidden in menus.

### Why Workspace-Level Environment Truth?
**Problem:** Capability, extensions, platform gates affect **all formula spaces**.

**Solution:** Put this information at workspace level (left rail footer) so it's:
- Always accessible
- Not duplicated per space
- Clear it applies globally

---

## Implementation Notes

### State Management
- **Workspace state:** Single source of truth for capability, extensions, platform
- **Formula-space state:** Per-space state for formula, result, retained runs, mode
- **Mode state:** Each space remembers last active mode
- **Drawer state:** Global, one drawer at a time

### Component Architecture
```
InformationArchitecture (shell)
├─ WorkspaceRail (workspace level)
├─ FormulaSpaceContextBar (formula-space level)
├─ ExploreMode (mode-specific canvas)
├─ InspectMode (mode-specific canvas)
├─ WorkbenchMode (mode-specific canvas)
└─ SecondaryDrawer (secondary details)
```

### Mode Switching Logic
1. User clicks mode button in context bar
2. `setCurrentMode('explore' | 'inspect' | 'workbench')`
3. Canvas component changes
4. Drawer closes automatically
5. Formula-space state persists

---

## Future Enhancements

### Multi-pane Support
- Split Explore canvas to show editor + inspector side-by-side
- Requires wider screens (2560px+)

### Custom Layouts
- Save preferred mode layouts per user
- Drag-and-drop panel customization

### Keyboard Shortcuts
- Cmd+1/2/3: Switch modes
- Cmd+I: Toggle drawer
- Cmd+K: Open completions

### Workspace Persistence
- Save workspace state to local storage
- Restore open formula spaces on app launch
- Remember last active mode per space

All enhancements maintain the three-mode architecture and ownership model.
