# DNA OneCalc - Focused Inspect Mode

## Overview

Focused Inspect Mode is a **semantic inspection environment** for understanding formula evaluation context, semantics, and execution meaning. It's designed for technically literate users who need to see how a formula was parsed, bound, and evaluated—not just what it returned.

**Core Feature:** **Formula Walk** - a structured, tree-aligned partial evaluation view showing subexpressions, bound names, intermediate values, and final result with collapsible nodes and clear state categories.

---

## Design Principles

### 1. Formula Walk is the Primary Surface
The **Formula Walk** (tree view) dominates the interface. This is where semantic inspection happens. Everything else supports this core feature.

### 2. Not a Log, Not a Dump
Inspect mode is **structured and purposeful**. It shows a formula-parse-tree-aligned view, not a chronological log or raw data dump. Every node has meaning.

### 3. Source Formula and Result Remain Visible
The source formula and result are **visible but secondary**. They provide context without stealing space from the walk.

### 4. State Categories Are Clear
Every node has a **visible state**: evaluated, bound, opaque, or blocked. Color-coded badges make it easy to understand what happened at each step.

### 5. Drawer for Deeper Detail
The right drawer provides **provenance chains, host context, and node detail** when you need to go deeper. Inspect mode doesn't depend entirely on the drawer—primary information is visible in the main canvas.

### 6. Technically Literate but Approachable
Inspect mode assumes you understand formulas and evaluation, but it doesn't require a PhD. Clear labels, descriptions, and visual hierarchy guide you through the semantic structure.

---

## Layout Structure

```
┌────────────────────────────────────────────────────────────────────┐
│  Global Top Bar (DNA OneCalc, search, help, settings)             │
├──────────┬─────────────────────────────────────────────────────────┤
│          │  Formula Space Context Bar                             │
│          │  (Space title, scenario policy, mode badge, tools)     │
│  Left    ├──────────┬──────────────────────┬──────────┬───────────┤
│  Rail    │          │                      │          │           │
│          │  Source  │   Formula Walk       │ Summary  │  Drawer   │
│  (Work-  │  Formula │   (Tree View)        │ (Parse,  │ (Provenan │
│  space,  │  +       │                      │  Bind,   │  ce, Con- │
│  spaces, │  Result  │   PRIMARY SURFACE    │  Eval,   │  text,    │
│  env)    │          │                      │  Host)   │  Node)    │
│          │  30%     │   45%                │  25%     │  37%      │
│          │          │                      │          │           │
├──────────┴──────────┴──────────────────────┴──────────┴───────────┤
│  Status Footer (inspection active, mode, versions)                │
└────────────────────────────────────────────────────────────────────┘
```

### Column Breakdown

**Without Drawer (Normal State):**
- **Source Formula + Result:** 30% width
- **Formula Walk:** 45% width (dominant)
- **Inspection Summary:** 25% width

**With Drawer (Provenance/Context/Node Open):**
- **Source Formula + Result:** 28% width
- **Formula Walk:** 35% width (compressed but still primary)
- **Inspection Summary:** 0% width (hidden)
- **Drawer:** 37% width

---

## Information Hierarchy

### Always Visible (Default State)

#### 1. Formula Walk Panel (Center, Dominant)
**The primary surface** - This is where semantic inspection happens

**Contains:**
- **Header:** "Formula Walk" label, explanation
- **State legend:** Evaluated (teal), Bound (amber), Opaque (gray), Blocked (terracotta)
- **Tree view:** Collapsible nodes showing formula structure
  - Each node displays:
    - **Expand/collapse icon** (chevron or dot)
    - **State badge** (evaluated/bound/opaque/blocked)
    - **Expression** (formula snippet)
    - **Value** (result or binding target)
    - **Description** (plain English explanation)
    - **Type** (Number, Array[5], etc.)
    - **Reason** (for opaque/blocked nodes)
  - **Indentation** shows hierarchy
  - **Click node** → opens detail in right drawer

**Visual priority:**
- Takes 45% of width (dominant)
- White background for clarity
- Color-coded state badges
- Monospace for expressions
- Collapsible tree structure

**Interaction:**
- Click chevron to expand/collapse node
- Click node to see detail in drawer
- Scroll to see full tree

**State Categories:**

**Evaluated (Teal #1e4d4a):**
- Node was successfully evaluated
- Value is the result of evaluation
- Example: `SUM(filtered) → 30`

**Bound (Amber #c88d2e):**
- Node is a name binding
- Value is what the name refers to
- Example: `values → SEQUENCE(10, 1, 1, 1)`

**Opaque (Gray #7a7568):**
- Node cannot be inspected deeply
- Host provides result but not detail
- Example: External function call

**Blocked (Terracotta #b84532):**
- Node evaluation was blocked
- Reason explains why
- Example: Circular reference, timeout

---

#### 2. Source Formula Panel (Left, Secondary)
**Context but not primary** - Read-only source for reference

**Contains:**
- **Header:** "Source Formula" label, "Edit" link to Explore mode
- **Note:** "Read-only • Switch to Explore mode to edit"
- **Compact formula display:** Monospace, syntax-aware (future), smaller than Explore mode
- **Result summary:** Large result value (3xl font, moss gradient), type/shape metadata
- **Quick action:** "Back to Explore" button

**Visual priority:**
- Takes 30% of width
- Parchment background (#faf7f1)
- Compact but readable
- Result is visible but not as large as in Explore mode

**Interaction:**
- Click "Edit" or "Back to Explore" → navigate to Explore mode
- Source formula is read-only (no editing in Inspect mode)

---

#### 3. Inspection Summary Panel (Right, Supporting)
**Nearby context** - Parse, bind, eval, host summaries

**Contains:**
- **Parse Summary:**
  - Status (Success/Error)
  - Tokens count
  - Functions count
  - Depth
  - Bindings count
- **Bind Summary:**
  - List of bound names (e.g., `values`, `filtered`)
  - Type and binding target for each
  - Amber-accented cards
- **Eval Summary:**
  - Status (Complete/Error)
  - Duration (1.2ms)
  - Nodes evaluated
  - Arrays created
  - Cache hits
- **Host Context:**
  - Profile (OC-H0)
  - OxFml version
  - OxFunc version
  - Locale
- **Active Flags:**
  - Freeze intermediate arrays (enabled/disabled)
  - Result caching (enabled/disabled)
  - Volatile functions (enabled/disabled)

**Visual priority:**
- Takes 25% of width
- Divided into sections with borders
- Moss green accents (#3e5238)
- Compact but readable
- Scrollable if needed

**Interaction:**
- View summaries at a glance
- Scroll to see all sections
- Click links in context bar to see more detail in drawer

---

### Nearby Secondary (Context Bar)

#### Inspect Context Bar
**Formula space level** - Scenario context, mode badge, drawer triggers

**Left section:**
- Formula space title ("LET Formula Analysis")
- Dirty indicator (dot)
- **Scenario policy (read-only):** "Policy: Deterministic" (small badge)

**Center:**
- **Mode badge:** "Inspect Mode" with eye icon (moss green accent)

**Right section:**
- **Provenance button:** Opens provenance chain drawer
- **Host Context button:** Opens host context detail drawer
- Separator
- Timing (1.2ms)
- Host profile (OC-H0)

**Interaction:**
- View scenario policy (read-only in Inspect mode)
- Click Provenance → opens provenance drawer
- Click Host Context → opens context drawer
- See timing and host profile at a glance

---

### On-Demand Detail (Right Drawer)

#### Provenance Drawer
**Value chain tracing** - Opens when you click "Provenance" in context bar

**Contains:**
- **Value Chain:** Step-by-step transformation from source to result
  - Each step shows:
    - **Node type:** Source, Binding, Transform, Result
    - **Expression:** Formula snippet
    - **Description:** What happened
    - **Color-coded dot:** State indicator
    - **Connecting line:** Visual flow
- **Metadata:** Transformations count, bindings count, array operations count

**Visual priority:**
- Slides in from right (37% width)
- Vertical timeline layout
- Color-coded steps (teal for evaluated, amber for bindings, moss for result)
- White background, organized sections

**Interaction:**
- Read through value chain from top to bottom
- See how value transformed from source to result
- Click X to close

---

#### Host Context Drawer
**Environment detail** - Opens when you click "Host Context" in context bar

**Contains:**
- **Host Profile:** Name (OC-H0), function set, array support
- **Scenario Policy:** Active policy (Deterministic), description
- **Scenario Flags:** Detailed list of enabled/disabled flags with explanations
- **Environment:** OxFml version, OxFunc version, locale, date format, decimal separator

**Visual priority:**
- Slides in from right (37% width)
- Moss green accents (#3e5238)
- Organized sections with borders
- Read-only information

**Interaction:**
- View full environment details
- Understand scenario configuration
- Click X to close

---

#### Node Detail Drawer
**Selected node inspection** - Opens when you click a node in Formula Walk

**Contains:**
- **Node Information:** State badge, expression, description, type, result, eval time
- **Function Signature:** Signature, arguments with types and descriptions
- **Inspection Status:** Whether full detail is available, or why opaque/blocked

**Visual priority:**
- Slides in from right (37% width)
- Amber accents (#c88d2e) for function signature
- Teal accents (#1e4d4a) for evaluated nodes
- Terracotta accents (#b84532) for blocked nodes

**Interaction:**
- Read detailed information about selected node
- Understand function signature
- See why node is opaque/blocked (if applicable)
- Click X to close

---

## Surface Placement Summary

| Surface | Always Visible | Nearby Secondary | On Demand |
|---------|----------------|------------------|-----------|
| **Formula Walk** | ✓ Center 45% | | Node detail drawer |
| **Source formula** | ✓ Left top | | Edit in Explore mode |
| **Result** | ✓ Left bottom | | |
| **Parse summary** | ✓ Right top | | |
| **Bind summary** | ✓ Right middle | | |
| **Eval summary** | ✓ Right middle | | |
| **Host context** | ✓ Right bottom | Context bar button | Full drawer |
| **Active flags** | ✓ Right bottom | | |
| **Provenance** | | Context bar button | Full drawer |
| **Node detail** | | Click node in walk | Full drawer |
| **Scenario policy** | | Context bar (read-only) | Edit in Explore mode |

---

## Responsive Behavior

### Desktop (1920x1080 or wider)
**Ideal state** - All three columns visible

- Source Formula + Result: 30% (~576px)
- Formula Walk: 45% (~864px)
- Inspection Summary: 25% (~480px)
- Drawer replaces Summary when open

### Narrow Desktop (1366x768)
**Compressed but functional**

- Source Formula + Result: 30% (~410px)
- Formula Walk: 45% (~615px)
- Inspection Summary: 25% (~341px)
- Drawer replaces Summary when open
- May need to compress node descriptions

### Tablet/Browser (1024x768)
**Two-column priority**

- Source Formula + Result: Hidden, accessible via button
- Formula Walk: 65% (~665px)
- Inspection Summary: 35% (~359px)
- Drawer overlays when opened

### Minimum Width
**1280px recommended** for optimal experience

Below 1280px:
- Hide Source Formula + Result panel (show in modal on demand)
- Formula Walk: 60%
- Inspection Summary: 40%
- Keep walk visible at all times

---

## Interaction Patterns

### Typical Workflow

1. **View formula walk** in center column (tree structure)
2. **Expand nodes** to see subexpressions
3. **Click node** to see detail in drawer
4. **Check summaries** in right column (parse, bind, eval)
5. **Click Provenance** to trace value chain
6. **Click Host Context** to see environment detail
7. **Back to Explore** to edit formula

### Exploring the Walk

1. **Start at root node** (top of tree, usually expanded by default)
2. **Click chevron** to expand/collapse children
3. **Read state badge** to understand node status
4. **View expression** to see formula snippet
5. **See value** to understand result or binding
6. **Read description** for plain English explanation
7. **Click node** to see full detail in drawer

### Tracing Provenance

1. **Click "Provenance"** button in context bar
2. **Drawer slides in** from right (Summary hides)
3. **Read value chain** from top to bottom
4. **See transformations:** Source → Binding → Transform → Result
5. **Understand flow:** Color-coded steps, connecting lines
6. **Click X** to close drawer (Summary returns)

### Understanding Host Context

1. **Click "Host Context"** button in context bar
2. **Drawer slides in** from right
3. **View host profile** (OC-H0, function set, array support)
4. **Check scenario policy** (Deterministic, Real-time, Full Random)
5. **See scenario flags** (freeze arrays, caching, volatile functions)
6. **Review environment** (versions, locale, formats)
7. **Click X** to close drawer

### Inspecting a Node

1. **Click a node** in Formula Walk
2. **Drawer slides in** from right with node detail
3. **Read node information** (state, expression, description, type, result)
4. **Review function signature** (if function call)
5. **Check inspection status** (available, opaque, blocked)
6. **Click X** to close drawer
7. **Click another node** to see different detail

---

## Visual Identity

### Color Palette (Warm Editorial, Moss Accent)

**Background tones:**
- `#faf7f1` - Parchment (main background)
- `#f7f3ea` - Light parchment (headers, rails)
- `#ede7da` - Warm smoke (top bar, footer)
- `#ffffff` - White (walk surface, cards)

**Primary accents (Inspect mode uses Moss):**
- `#3e5238` - Moss (Inspect mode, primary actions, host context)
- `#1e4d4a` - Oxidized teal (Evaluated state)
- `#c88d2e` - Amber brass (Bound state, node detail)
- `#b84532` - Terracotta (Blocked state)
- `#7a7568` - Warm gray (Opaque state)

**Text:**
- `#1f1c17` - Espresso ink (primary text)
- `#7a7568` - Warm gray (secondary text)

**Borders:**
- `#1f1c17/10` - 10% espresso (subtle borders)
- `#1f1c17/15` - 15% espresso (input borders)

### State Badge Colors

**Evaluated:**
- Background: `bg-[#1e4d4a]/5`
- Border: `border-[#1e4d4a]/20`
- Text: `text-[#1e4d4a]`
- Icon: CheckCircle2

**Bound:**
- Background: `bg-[#c88d2e]/5`
- Border: `border-[#c88d2e]/20`
- Text: `text-[#c88d2e]`
- Icon: Circle

**Opaque:**
- Background: `bg-[#7a7568]/5`
- Border: `border-[#7a7568]/20`
- Text: `text-[#7a7568]`
- Icon: Lock

**Blocked:**
- Background: `bg-[#b84532]/5`
- Border: `border-[#b84532]/20`
- Text: `text-[#b84532]`
- Icon: AlertCircle

### Typography

**Headers:** System sans-serif, 600 weight, tight tracking
**Body:** System sans-serif, 400 weight, relaxed leading
**Monospace:** System monospace (Monaco, Menlo, Consolas)
- Node expressions: 12px (xs), leading-relaxed
- Result value: 36px (3xl)
- Code snippets: 12px (xs)

### Spacing

**Node padding:**
- Node row: py-2.5 px-3
- Nested indent: 24px per level
- Gaps: gap-3 (12px)

**Panel padding:**
- Headers: px-4 py-3
- Content: p-4
- Cards: p-3

### Borders & Shadows

**Borders:**
- Subtle: border border-[#1f1c17]/10
- State badge: border (with state color)
- Sections: border-b border-[#1f1c17]/10

**Shadows:**
- Drawer: shadow-2xl
- Timeline dots: shadow

---

## Component Architecture

### FocusedInspectMode (Shell)
**Root component** - Manages drawer state, layout orchestration

**Props:** None
**State:** 
- `drawerOpen` (null | 'provenance' | 'context' | 'node')
- `selectedNode` (string | null)

**Children:**
- Global top bar (DNA OneCalc branding)
- WorkspaceRail (left)
- InspectContextBar (formula space level)
- SourceFormulaPanel (left column)
- FormulaWalkPanel (center column)
- InspectionSummaryPanel (right column)
- InspectDrawer (conditional, right drawer)
- Status footer

---

### InspectContextBar
**Formula space level** - Scenario context, mode badge, drawer triggers

**Props:** `onOpenDrawer(drawer: 'provenance' | 'context')`
**State:** None

**Features:**
- Formula space title + dirty indicator
- Scenario policy (read-only badge)
- Mode badge (Inspect Mode with eye icon)
- Provenance button
- Host Context button
- Timing + host profile

---

### SourceFormulaPanel
**Source context** - Read-only formula and result

**Props:** `onBackToExplore()`
**State:** None

**Features:**
- Source formula header (with "Edit" link)
- Read-only note
- Compact formula display (monospace, smaller)
- Result summary (large value, type/shape)
- "Back to Explore" button

**Interaction:**
- Click "Edit" or "Back to Explore" → calls `onBackToExplore()`

---

### FormulaWalkPanel
**The primary surface** - Tree-aligned semantic inspection

**Props:** `onSelectNode(nodeId: string)`
**State:** 
- `expandedNodes` (Set<string>)

**Features:**
- Formula Walk header + explanation
- State legend (Evaluated, Bound, Opaque, Blocked)
- Tree view with collapsible nodes
- Each node shows:
  - Expand/collapse icon
  - State badge
  - Expression
  - Value
  - Description
  - Type
  - Reason (if blocked/opaque)
- Indentation for hierarchy
- Click to expand/collapse
- Click node to select

**Interaction:**
- Click chevron → toggles node expansion
- Click node → calls `onSelectNode(nodeId)`
- Scroll to see full tree

---

### InspectionSummaryPanel
**Supporting context** - Parse, bind, eval, host summaries

**Props:** None
**State:** None

**Features:**
- Parse Summary (status, tokens, functions, depth, bindings)
- Bind Summary (list of bound names with types)
- Eval Summary (status, duration, nodes, arrays, cache hits)
- Host Context (profile, versions, locale)
- Active Flags (list of enabled/disabled flags)

**Interaction:**
- View summaries at a glance
- Scroll to see all sections

---

### InspectDrawer
**Deep detail drawer** - Provenance, Context, Node

**Props:** 
- `type` (null | 'provenance' | 'context' | 'node')
- `onClose()`

**State:** None

**Features:**
- Drawer header (title, description, close button)
- Content area (ProvenanceContent, ContextContent, or NodeContent)
- Slides in from right, replaces Inspection Summary

**Behavior:**
- Renders only when `type` is not null
- Click X → calls `onClose()`

**Content Components:**

**ProvenanceContent:**
- Value chain (timeline of transformations)
- Metadata (transformations, bindings, array ops)

**ContextContent:**
- Host profile (name, function set, array support)
- Scenario policy (active policy, description)
- Scenario flags (detailed list)
- Environment (versions, locale, formats)

**NodeContent:**
- Node information (state, expression, description, type, result)
- Function signature (signature, arguments)
- Inspection status (available, opaque, blocked)

---

## State Management

### Formula-Space Level State
**Managed by FocusedInspectMode**

- `drawerOpen` - Which drawer is open (null | 'provenance' | 'context' | 'node')
- `selectedNode` - Which node is selected (string | null)
- `walkData` - Formula walk tree (managed by FormulaWalkPanel)
- `expandedNodes` - Which nodes are expanded (managed by FormulaWalkPanel)

### Component Local State

- **FormulaWalkPanel:** `expandedNodes` (Set<string>)
- **InspectContextBar:** None (all interactions trigger drawer)

---

## Formula Walk Data Structure

### WalkNode Interface

```typescript
interface WalkNode {
  id: string;                    // Unique identifier
  expression: string;            // Formula snippet (e.g., "SUM(filtered)")
  state: 'evaluated' | 'bound' | 'opaque' | 'blocked';
  value?: string;                // Result or binding target
  type?: string;                 // Data type (e.g., "Number", "Array[5]")
  children?: WalkNode[];         // Child nodes
  description?: string;          // Plain English explanation
  reason?: string;               // Why opaque/blocked (if applicable)
}
```

### Example Walk Data

```typescript
const walkData: WalkNode = {
  id: 'root',
  expression: '=LET(...)',
  state: 'evaluated',
  value: '30',
  type: 'Number',
  description: 'Root expression',
  children: [
    {
      id: 'let',
      expression: 'LET(values, SEQUENCE(...), filtered, FILTER(...), SUM(...))',
      state: 'evaluated',
      value: '30',
      type: 'Number',
      description: 'LET binding with named variables',
      children: [
        {
          id: 'values-name',
          expression: 'values',
          state: 'bound',
          value: 'SEQUENCE(10, 1, 1, 1)',
          description: 'Name binding: values',
        },
        {
          id: 'values-value',
          expression: 'SEQUENCE(10, 1, 1, 1)',
          state: 'evaluated',
          value: '[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]',
          type: 'Array[10]',
          description: 'Sequence generation',
        },
        // ... more nodes
      ],
    },
  ],
};
```

---

## Future Enhancements

### Walk Improvements
- **Syntax highlighting:** Color-code expressions in tree
- **Value preview:** Hover to see full value (for arrays)
- **Search in walk:** Find nodes by expression or value
- **Filter by state:** Show only evaluated/bound/opaque/blocked nodes
- **Copy node:** Copy expression or value to clipboard

### Provenance Enhancements
- **Value diff:** Show how value changed at each step
- **Transformation preview:** See input vs output for each transform
- **Dependency graph:** Visual graph of how values flow

### Context Improvements
- **Compare scenarios:** See how different policies affect evaluation
- **Flag impact:** Explain why each flag is enabled/disabled
- **Performance profiling:** See timing breakdown by node

### Layout Flexibility
- **Resizable columns:** Drag to resize walk, summary, source panels
- **Detach drawer:** Pop out drawer to separate window
- **Persistent preferences:** Remember expanded nodes, drawer state

### Accessibility
- **Keyboard navigation:** Tab through nodes, expand/collapse with Enter
- **Screen reader labels:** ARIA labels for tree structure
- **High contrast mode:** Support system high contrast settings

---

## Design Rationale

### Why Formula Walk is Central?

**Problem:** Other inspection views (logs, dumps, raw data) don't show the *structure* of evaluation. You see individual values but not how they relate to the formula.

**Solution:** Formula Walk is a **tree-aligned view** that matches the parse tree. Each node corresponds to a subexpression. You can see the hierarchical relationship and understand how the formula was evaluated step-by-step.

**Rationale:** The walk is the **semantic inspection**. Everything else (summaries, provenance, context) supports understanding the walk.

### Why Four State Categories?

**Problem:** Binary states (success/error) don't capture the nuance of formula evaluation. Some nodes can't be inspected deeply, others are blocked.

**Solution:** Four states:
- **Evaluated:** Successfully evaluated, value is the result
- **Bound:** Name binding, value is what the name refers to
- **Opaque:** Host provides result but not detail (black box function)
- **Blocked:** Evaluation was blocked (error, timeout, circular reference)

**Rationale:** These states cover all possible node outcomes. Color-coding makes them instantly recognizable.

### Why Source Formula is Read-Only?

**Problem:** If formula is editable in Inspect mode, user might expect to re-evaluate and see updated walk. This creates confusion about mode purpose.

**Solution:** Source formula is **read-only in Inspect mode**. "Back to Explore" button makes it clear where to edit.

**Rationale:** Inspect mode is for *understanding*, not *editing*. Clear separation of concerns.

### Why Drawer Replaces Summary?

**Problem:** If drawer overlays summary, user can't see walk and detail simultaneously. If we compress all three columns + drawer, everything is too narrow.

**Solution:** Drawer **replaces Summary panel**. Walk and Source remain visible. This keeps walk at usable width while providing deep detail.

**Rationale:** Summary is useful at a glance, but when you need detail (provenance, context, node), you don't need the summary. Trade-off keeps walk usable.

### Why Provenance as Value Chain?

**Problem:** Traditional call stacks or execution logs are chronological but don't show *value transformation*. You see function calls but not how the value changed.

**Solution:** Provenance shows a **value chain**: Source → Binding → Transform → Result. Each step explains what happened to the value, not just what function was called.

**Rationale:** Users care about *how the value got here*, not just *what functions were called*. Value chain answers that question.

---

## Key Differences from Explore Mode

### Explore Mode (Formula Authoring)
- **Purpose:** Create and test formulas
- **Primary surface:** Formula editor (editable)
- **Result:** Large, prominent
- **Completions:** Always visible
- **Diagnostics:** Inline below editor
- **Workflow:** Edit → Evaluate → See result → Adjust

### Inspect Mode (Semantic Understanding)
- **Purpose:** Understand how formula was evaluated
- **Primary surface:** Formula Walk (tree view)
- **Result:** Visible but secondary
- **Source formula:** Read-only, compact
- **Summaries:** Parse, bind, eval, host
- **Workflow:** View walk → Expand nodes → Trace provenance → Understand semantics

**When to use Explore Mode:**
- You're writing a new formula
- You're debugging a formula
- You want to test different inputs
- You need to edit the formula

**When to use Inspect Mode:**
- You want to understand how a formula works
- You need to see intermediate values
- You want to trace provenance
- You're investigating why a result is unexpected

---

## Conclusion

Focused Inspect Mode is a **semantic inspection environment** that prioritizes understanding over editing. The **Formula Walk** provides a structured, tree-aligned view of formula evaluation, showing subexpressions, bindings, intermediate values, and state categories. Supporting summaries, provenance tracing, and host context detail help you understand not just *what* the formula returned, but *how* and *why*.

**Core principle:** Show the semantic structure of formula evaluation, not a chronological log or raw data dump. Make the walk primary, everything else supporting.
