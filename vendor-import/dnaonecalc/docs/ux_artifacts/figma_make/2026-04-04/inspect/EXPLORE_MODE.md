# DNA OneCalc - Focused Explore Mode

## Overview

Focused Explore Mode is a refined single-mode layout optimized for **formula authoring and function discovery**. It keeps the formula editor, result, and function reference visible together in ordinary use, with secondary controls accessible but not dominating the interface.

---

## Design Principles

### 1. Formula and Result Visible Together
The editor and result are **always visible side-by-side** in normal use. You never have to choose between seeing your formula and seeing the output.

### 2. Completions and Help Nearby
Function reference (completions + current help) is **immediately adjacent to the editor**, not in a distant drawer. This supports rapid function discovery and signature lookup without losing context.

### 3. Diagnostics Stay Close to Editor
Parse errors and validation messages appear **directly below the editor**, not in a separate inspector panel. If there's a problem with your formula, you see it where the problem is.

### 4. Secondary Controls on Demand
Formatting, scenario flags, and advanced settings live in a **right drawer** that slides in when needed. These are important but not primary to the editing flow.

### 5. Optimized for Repeated Exploration
The layout assumes you're **exploring many formulas quickly**: edit, evaluate, see result, adjust, repeat. Everything you need for this loop is visible without extra clicks.

---

## Layout Structure

```
┌────────────────────────────────────────────────────────────────────┐
│  Global Top Bar (DNA OneCalc, search, help, settings)             │
├──────────┬─────────────────────────────────────────────────────────┤
│          │  Formula Space Context Bar                             │
│          │  (Space title, scenario policy, mode badge, tools)     │
│  Left    ├──────────────┬──────────────┬──────────────┬───────────┤
│  Rail    │              │              │              │           │
│          │   Formula    │   Result     │  Completion  │  Drawer   │
│  (Work-  │   Editor     │   + Array    │  + Help      │ (slides   │
│  space,  │              │   Preview    │              │  in on    │
│  spaces, │   40%        │   35%        │   25%        │  demand)  │
│  env)    │              │              │              │  35%      │
│          │              │              │              │           │
├──────────┴──────────────┴──────────────┴──────────────┴───────────┤
│  Status Footer (ready state, mode, versions)                      │
└────────────────────────────────────────────────────────────────────┘
```

### Column Breakdown

**Without Drawer (Normal State):**
- **Formula Editor:** 40% width
- **Result + Array Preview:** 35% width
- **Completion + Help:** 25% width

**With Drawer (Formatting/Settings Open):**
- **Formula Editor:** 38% width
- **Result + Array Preview:** 27% width
- **Completion + Help:** 0% width (hidden)
- **Drawer:** 35% width

---

## Information Hierarchy

### Always Visible (Default State)

#### 1. Formula Editor Panel (Left)
**Dominant surface** - This is where the work happens

**Contains:**
- **Editor header:** Formula label, line count, token count, function count, Evaluate button
- **Multiline editor:** Line numbers, syntax area (not full syntax highlighting yet), cursor fidelity
- **Inline diagnostics:** Success/error banner directly below editor

**Visual priority:**
- Large, white background
- Border-right separation
- Editor takes 40% of width
- Serious editing surface, not a textarea, not a full IDE

**Interaction:**
- Type formula
- Use arrow keys, selection
- Click Evaluate button (or Cmd+Enter)
- See diagnostics immediately below

---

#### 2. Result Panel (Center)
**Prominent feedback** - See the output of your formula

**Contains:**
- **Result header:** Result label, evaluation status (✓ Evaluated 1.2ms)
- **Result value display:** Large number/value, dominant gradient background, type/shape metadata
- **Effective display (inline):** Current formatting settings (Format, Precision, Style, Color) with "Edit" link
- **Array preview:** Intermediate arrays from evaluation (when relevant)

**Visual priority:**
- Large result value (6xl font, gradient background)
- Effective display is visible but not dominant
- Array preview in scrollable area below
- Takes 35% of width

**Interaction:**
- View result
- Click "Edit" on Effective Display → opens Formatting drawer
- Expand array preview for more detail
- Scroll to see intermediate arrays

---

#### 3. Completion + Help Panel (Right)
**Function reference** - Discover and learn functions

**Contains:**
- **Function reference header:** Search box, function count
- **Completion list:** Scrollable list of functions (20 visible), click to select
- **Current help (bottom):** Selected function documentation (signature, description, arguments, metadata)

**Visual priority:**
- Compact but readable
- Current help has amber accent background (from-[#c88d2e]/5)
- Takes 25% of width
- Split vertically: completions top, help bottom (fixed height)

**Interaction:**
- Search functions
- Click function to see help
- Scroll completion list
- Read current help for selected function

---

### Nearby Secondary (Context Bar)

#### Formula Space Context Bar
**Scenario-level controls** - Not dominant, but easily accessible

**Left section:**
- Formula space title ("LET Formula Analysis")
- Dirty indicator (dot)
- **Scenario policy dropdown:** Deterministic / Real-time / Full Random

**Center:**
- **Mode badge:** "Explore Mode" with teal dot

**Right section:**
- **Formatting button:** Opens Formatting drawer
- **Settings button:** Opens Scenario Settings drawer
- Separator
- Timing (1.2ms)
- Host profile (OC-H0)

**Interaction:**
- Click scenario policy → dropdown with three options
- Click Formatting → opens right drawer
- Click Settings → opens right drawer
- See timing and host profile at a glance

---

### On-Demand Secondary (Right Drawer)

#### Formatting Drawer
**Display formatting controls** - Opens when you click "Formatting" or "Edit" on Effective Display

**Contains:**
- **Number format:** Format code input, decimal places dropdown, thousands separator checkbox
- **Conditional formatting:** Result color by value, icon set, data bars, color scale (checkboxes with descriptions)
- **Style presets:** Default, Currency, Percentage, Scientific (button grid)

**Visual priority:**
- Slides in from right (35% width)
- Replaces Completion + Help panel
- Formula and Result remain visible
- White background, organized sections

**Interaction:**
- Edit format code
- Select decimal places
- Toggle conditional formatting rules
- Choose style presets
- Click X to close

---

#### Scenario Settings Drawer
**Scenario-affecting host flags** - Opens when you click "Settings" in context bar

**Contains:**
- **Scenario flags (gradient box):** Allow volatile functions, freeze intermediate arrays, enable result caching, strict evaluation mode
- **Evaluation settings:** Max iterations, timeout (ms)
- **Warning banner:** "Scenario flags affect evaluation behavior. Changes apply to the current formula space only."

**Visual priority:**
- Slides in from right (35% width)
- Moss green accent (#3e5238)
- Checkboxes with descriptions
- Warning at bottom

**Interaction:**
- Toggle scenario flags
- Adjust evaluation settings
- Read warning
- Click X to close

---

## Surface Placement Summary

| Surface | Always Visible | Nearby Secondary | On Demand |
|---------|----------------|------------------|-----------|
| **Formula editor** | ✓ Left 40% | | |
| **Result** | ✓ Center top | | |
| **Effective display** | ✓ Center inline | | Full editor in drawer |
| **Array preview** | ✓ Center bottom | | Expand action |
| **Diagnostics** | ✓ Below editor | | Parse tree link |
| **Completion list** | ✓ Right top | | |
| **Current help** | ✓ Right bottom | | |
| **Scenario policy** | | Context bar dropdown | |
| **Formatting entry** | | Context bar button | Full drawer |
| **Settings entry** | | Context bar button | Full drawer |
| **Timing + host** | | Context bar right | |

---

## Responsive Behavior

### Desktop (1920x1080 or wider)
**Ideal state** - All three columns visible

- Formula Editor: 40% (~768px)
- Result: 35% (~672px)
- Completion + Help: 25% (~480px)
- Drawer replaces Completion + Help when open

### Narrow Desktop (1366x768)
**Compressed but functional**

- Formula Editor: 40% (~546px)
- Result: 35% (~478px)
- Completion + Help: 25% (~342px)
- Drawer replaces Completion + Help when open
- May need to hide array preview or make it collapsible

### Tablet/Browser (1024x768)
**Two-column priority**

- Formula Editor: 50% (~512px)
- Result: 50% (~512px)
- Completion + Help: Hidden, accessible via button
- Drawer overlays when opened

### Minimum Width
**1280px recommended** for optimal experience

Below 1280px:
- Consider showing Completion + Help as a bottom drawer
- Keep Formula + Result side-by-side
- Effective display becomes collapsible

---

## Interaction Patterns

### Typical Workflow

1. **Type formula** in editor (left column)
2. **See diagnostics** immediately below editor
3. **Click Evaluate** (or Cmd+Enter)
4. **View result** in center column (large value)
5. **Check array preview** in center column (if arrays present)
6. **Adjust formatting** if needed (click "Edit" → Formatting drawer)
7. **Search functions** in right column if needed
8. **Read help** for selected function in right column

### Scenario Policy Change

1. **Click "Deterministic"** dropdown in context bar
2. **Select policy:** Deterministic, Real-time, or Full Random
3. **Dropdown closes** automatically
4. **Re-evaluate** formula to see effect

### Formatting Change

1. **Click "Formatting"** button in context bar OR "Edit" on Effective Display
2. **Drawer slides in** from right (Completion + Help hides)
3. **Edit format settings** (number format, conditional formatting, style presets)
4. **Changes apply** immediately to result
5. **Click X** to close drawer (Completion + Help returns)

### Settings Change

1. **Click "Settings"** button in context bar
2. **Drawer slides in** from right
3. **Toggle scenario flags** (volatile functions, array freezing, caching, strict mode)
4. **Adjust evaluation settings** (max iterations, timeout)
5. **Click X** to close drawer

---

## Visual Identity

### Color Palette (Warm Editorial)

**Background tones:**
- `#faf7f1` - Parchment (main background)
- `#f7f3ea` - Light parchment (headers, rails)
- `#ede7da` - Warm smoke (top bar, footer)
- `#ffffff` - White (editor surface, cards)

**Primary accents:**
- `#1e4d4a` - Oxidized teal (Explore mode, primary actions)
- `#3e5238` - Moss (Settings, secondary actions)
- `#c88d2e` - Amber brass (Current help, warnings)
- `#b84532` - Terracotta (Not used in Explore mode)

**Text:**
- `#1f1c17` - Espresso ink (primary text)
- `#7a7568` - Warm gray (secondary text)

**Borders:**
- `#1f1c17/10` - 10% espresso (subtle borders)
- `#1f1c17/15` - 15% espresso (input borders)

### Typography

**Headers:** System sans-serif, 600 weight, tight tracking
**Body:** System sans-serif, 400 weight, relaxed leading
**Monospace:** System monospace (Monaco, Menlo, Consolas)
- Formula editor: 15px, leading-7
- Result value: 6xl (60px)
- Code snippets: 12px

### Spacing

**Padding:**
- Headers: px-4 py-3
- Content: p-6
- Cards: p-4
- Tight: p-2.5

**Gaps:**
- Section: gap-6
- Inline: gap-3
- Tight: gap-2

### Borders & Shadows

**Borders:**
- Subtle: border border-[#1f1c17]/10
- Input: border border-[#1f1c17]/15
- Accent: border-2 border-[#1e4d4a]/20

**Shadows:**
- Drawer: shadow-2xl
- Button: shadow-sm
- Cards: No shadow (borders only)

---

## Component Architecture

### FocusedExploreMode (Shell)
**Root component** - Manages drawer state, layout orchestration

**Props:** None
**State:** `drawerOpen` (null | 'formatting' | 'settings')

**Children:**
- Global top bar (DNA OneCalc branding)
- WorkspaceRail (left)
- ExploreContextBar (formula space level)
- FormulaEditorPanel (left column)
- ResultPanel (center column)
- CompletionHelpPanel (right column)
- ExploreDrawer (conditional, right drawer)
- Status footer

---

### ExploreContextBar
**Formula space level** - Scenario controls, mode badge, tools

**Props:** `onOpenDrawer(drawer: 'formatting' | 'settings')`
**State:** `showPolicy` (boolean for dropdown)

**Features:**
- Formula space title + dirty indicator
- Scenario policy dropdown
- Mode badge (Explore Mode)
- Formatting button
- Settings button
- Timing + host profile

---

### FormulaEditorPanel
**Formula editing surface** - Multiline editor with diagnostics

**Props:** `onEvaluate()`
**State:** `formula` (string)

**Features:**
- Editor header (line count, token count, function count)
- Line numbers (left gutter)
- Multiline textarea (font-mono, 15px, leading-7)
- Evaluate button (primary action)
- Inline diagnostics banner (success/error)

**Future enhancements:**
- Syntax highlighting
- Autocomplete dropdown
- Error squiggles
- Keyboard shortcuts

---

### ResultPanel
**Result display** - Large value, effective display, array preview

**Props:** `onOpenFormatting()`
**State:** None (receives data from parent)

**Features:**
- Result header (evaluation status)
- Large result value (6xl, gradient background)
- Type/shape metadata
- Effective display (inline, collapsible)
- Array preview (scrollable, intermediate arrays)

**Interaction:**
- Click "Edit" on Effective Display → calls `onOpenFormatting()`
- Expand array preview for more detail

---

### CompletionHelpPanel
**Function reference** - Searchable completions + current help

**Props:** None
**State:** `searchQuery` (string), `selectedFunction` (string)

**Features:**
- Search input (filters completions)
- Completion list (scrollable, 20 visible)
- Current help (bottom section, fixed height)
- Function signature, description, arguments, metadata

**Interaction:**
- Type in search box → filters completions
- Click function → selects it, updates help
- Scroll completion list

---

### ExploreDrawer
**Secondary detail drawer** - Formatting or Settings

**Props:** `type` (null | 'formatting' | 'settings'), `onClose()`
**State:** None

**Features:**
- Drawer header (title, description, close button)
- Content area (FormattingContent or SettingsContent)
- Slides in from right, replaces Completion + Help

**Behavior:**
- Renders only when `type` is not null
- Click X → calls `onClose()`
- Esc key → calls `onClose()` (future)

---

## State Management

### Formula-Space Level State
**Managed by FocusedExploreMode**

- `drawerOpen` - Which drawer is open (null | 'formatting' | 'settings')
- `formula` - Current formula text (managed by FormulaEditorPanel)
- `result` - Evaluation result (future: managed by parent)
- `effectiveDisplay` - Current formatting settings (future)
- `scenarioPolicy` - Deterministic / Real-time / Full Random (future)
- `scenarioFlags` - Host flags (future)

### Component Local State

- **ExploreContextBar:** `showPolicy` (dropdown visibility)
- **FormulaEditorPanel:** `formula` (editor content)
- **CompletionHelpPanel:** `searchQuery`, `selectedFunction`

---

## Future Enhancements

### Editor Improvements
- **Syntax highlighting:** Color-code functions, operators, literals
- **Autocomplete dropdown:** Show completions inline as you type
- **Error squiggles:** Underline errors directly in editor
- **Keyboard shortcuts:** Cmd+Enter to evaluate, Cmd+/ for comment

### Result Enhancements
- **Result history:** See previous results, compare values
- **Copy result:** One-click copy to clipboard
- **Result formatting preview:** See formatted value in Effective Display

### Function Reference
- **Recent functions:** Show recently used functions at top
- **Favorites:** Pin frequently used functions
- **Examples:** Show example usage in current help

### Layout Flexibility
- **Resizable columns:** Drag to resize editor, result, completion panels
- **Collapsible array preview:** Hide when not needed
- **Persistent drawer:** Option to keep drawer open while working

### Accessibility
- **Keyboard navigation:** Tab through all controls
- **Screen reader labels:** ARIA labels for all interactive elements
- **High contrast mode:** Support system high contrast settings

---

## Implementation Notes

### Column Width Calculation

**Normal state (no drawer):**
- Formula: 40% of remaining width after left rail
- Result: 35% of remaining width
- Completion: 25% of remaining width

**Drawer open:**
- Formula: 38% (slightly compressed)
- Result: 27% (compressed)
- Completion: 0% (hidden)
- Drawer: 35% (replaces completion)

**Why hide Completion + Help when drawer opens?**
- Keeps formula and result visible (primary workflow)
- Drawer provides enough space for formatting controls
- User can close drawer to restore completion list

### Drawer Behavior

**Opening:**
1. User clicks "Formatting" or "Settings" in context bar
2. `setDrawerOpen('formatting' | 'settings')`
3. Completion + Help panel width animates to 0%
4. Drawer width animates to 35%

**Closing:**
1. User clicks X in drawer header
2. `setDrawerOpen(null)`
3. Drawer width animates to 0%
4. Completion + Help panel width animates to 25%

**Transition:** `transition-all duration-300` for smooth animation

---

## Design Rationale

### Why Three Columns Instead of Tabs?

**Problem:** Tabs hide critical information. If result is in a tab, you can't see it while editing.

**Solution:** Three visible columns keep formula, result, and function reference visible together. This supports the rapid iteration loop: edit → evaluate → see result → adjust.

### Why Right Drawer for Formatting?

**Problem:** Formatting is important but not used constantly. If it's always visible, it takes space from primary workflow.

**Solution:** Formatting drawer slides in on demand. When you need it, it's one click away. When you don't, it's not taking space.

### Why Hide Completion + Help When Drawer Opens?

**Problem:** If we compress all three columns + drawer, everything becomes too narrow to use.

**Solution:** Hide Completion + Help when drawer is open. This keeps formula and result at usable widths. When user closes drawer, completion list returns.

**Rationale:** Formatting and completion are rarely used simultaneously. When formatting, you're focused on the result, not discovering new functions.

### Why Inline Effective Display?

**Problem:** If all formatting controls are hidden in a drawer, you can't see current settings at a glance.

**Solution:** Show current formatting settings inline on the result panel. Click "Edit" to open full formatting drawer. This gives you visibility without clutter.

### Why Diagnostics Below Editor?

**Problem:** If diagnostics are in a separate panel, you have to look away from the formula to see errors.

**Solution:** Show diagnostics directly below the editor. Parse errors, validation messages, and success state are immediately visible where the formula is.

---

## Key Differences from Three-Mode Architecture

The **Focused Explore Mode** is a single-mode refinement, distinct from the full three-mode architecture:

### Three-Mode Architecture (Information Architecture)
- **Three modes:** Explore, Inspect, Workbench
- **Mode switcher:** Toggle between modes in context bar
- **Canvas changes:** Completely different layout per mode
- **Right drawer:** Secondary details specific to each mode

### Focused Explore Mode (This)
- **One mode:** Explore only
- **No mode switcher:** Just Explore Mode badge
- **Fixed layout:** Three columns always visible
- **Right drawer:** Formatting and Settings only

**When to use Focused Explore Mode:**
- You're building Explore mode in isolation
- You want a simpler, single-purpose layout
- You're not ready to implement Inspect and Workbench

**When to use Three-Mode Architecture:**
- You need all three task modes (Explore, Inspect, Workbench)
- You want a comprehensive formula analysis tool
- You're ready to handle mode-specific layouts

---

## Conclusion

Focused Explore Mode prioritizes formula authoring and function discovery. It keeps the editor, result, and function reference visible together, with secondary controls (formatting, scenario flags) accessible but not dominating the interface. This layout is optimized for rapid exploration: edit, evaluate, see result, adjust, repeat.

**Core principle:** Everything you need to explore formulas is visible by default. Everything else is one click away.
