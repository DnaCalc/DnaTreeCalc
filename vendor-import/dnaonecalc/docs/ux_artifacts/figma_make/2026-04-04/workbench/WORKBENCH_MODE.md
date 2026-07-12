# DNA OneCalc - Focused Workbench Mode

## Overview

Focused Workbench Mode is an **evidence workbench** for retained evidence, comparison, replay, observation, widening, and handoff. It's designed for understanding what matched, what differed, what is lossy, what is blocked, and what the next action is.

**Core Purpose:** Provide a structured environment for comparing runs, building evidence bundles, and handing off traceable artifacts—not a generic admin page or debugging console, but a purpose-built workbench for formula evidence.

---

## Design Principles

### 1. Evidence Workbench, Not Admin Page
This is a **workbench for serious evidence work**. Every element serves the purpose of understanding, comparing, retaining, or handing off formula evaluation evidence.

### 2. Comparison Outcome is Clear
You should immediately understand **what matched, what differed, and what is blocked**. The comparison outcome is the first thing you see in the main panel.

### 3. Replay Lineage is Visually Strong
The **timeline of runs** is not a list—it's a visual lineage showing how the formula was evaluated across time. Color-coded, with clear progression from initial to latest.

### 4. Evidence is Traceable
Every artifact has an **ID and clear identity**: scenario, run, observation, comparison, witness, handoff. These are related but distinct objects that can be referenced and traced.

### 5. Reliability is Explicit
A **reliability badge** (percentage + progress bar) tells you the confidence level of this evidence. High reliability means consistent runs; low reliability signals variance.

### 6. Actions are Primary, Not Hidden
**Export, Retain, Widen, Handoff** actions are prominent. This mode is about *doing something* with the evidence, not just viewing it.

### 7. Blocked Dimensions are Visible
If something **cannot be compared** (opaque function, external API, etc.), it's shown clearly with reasons. Transparency about limitations.

---

## Layout Structure

```
┌────────────────────────────────────────────────────────────────────┐
│  Global Top Bar (DNA OneCalc, search, help, settings)             │
├──────────┬─────────────────────────────────────────────────────────┤
│          │  Formula Space Context Bar                             │
│          │  (Space title, timestamp, mode badge, witness/handoff) │
│  Left    ├──────────┬──────────────────────┬──────────┬───────────┤
│  Rail    │          │                      │          │           │
│          │ Evidence │  Main Workbench      │ Actions  │  Drawer   │
│  (Work-  │ Bundle   │  (Comparison,        │ (Export, │ (Witness, │
│  space,  │ (Source, │   Lineage,           │  Retain, │  Handoff) │
│  spaces, │  Relia-  │   Observation)       │  Widen,  │           │
│  env)    │  bility, │                      │  Handoff,│  37%      │
│          │  Arti-   │   PRIMARY SURFACE    │  Blocked)│           │
│          │  facts)  │                      │          │           │
│          │  25%     │   50%                │  25%     │           │
│          │          │                      │          │           │
├──────────┴──────────┴──────────────────────┴──────────┴───────────┤
│  Status Footer (evidence active, mode, versions)                  │
└────────────────────────────────────────────────────────────────────┘
```

### Column Breakdown

**Without Drawer (Normal State):**
- **Evidence Bundle:** 25% width
- **Main Workbench:** 50% width (dominant)
- **Actions & Blocked Dimensions:** 25% width

**With Drawer (Witness/Handoff Open):**
- **Evidence Bundle:** 23% width
- **Main Workbench:** 40% width (compressed but still primary)
- **Actions:** 0% width (hidden)
- **Drawer:** 37% width

---

## Information Hierarchy

### Always Visible (Default State)

#### 1. Main Workbench Panel (Center, Dominant)
**The primary surface** - Comparison outcome, replay lineage, observation envelope

**Contains:**
- **Comparison Outcome Section:**
  - **Three metric cards:**
    - **Matched** (teal): Dimensions that matched across runs
    - **Differed** (amber): Dimensions with variance
    - **Blocked** (terracotta): Dimensions unavailable for comparison
  - **Outcome Detail:** List of specific dimensions (result value, type stability, array shapes, evaluation path, timing variance)
  
- **Replay Lineage Section:**
  - **Timeline of Runs:** Vertical timeline with color-coded nodes
    - Initial run (moss green)
    - Replay runs (teal)
    - Latest run (terracotta, pulsing)
  - Each run shows:
    - Run number + label (Initial/Replay/Latest)
    - Timestamp
    - Result value
    - Execution time
    - Policy (Deterministic/Real-time/etc.)
  - Connecting lines show progression
  
- **Observation Envelope Section:**
  - **Captured State:** Grid of metadata
    - Formula hash
    - Host profile
    - OxFml version
    - Scenario policy
    - Frozen timestamp
    - Run count

**Visual priority:**
- Takes 50% of width (dominant)
- White background
- Organized sections with vertical rule markers (terracotta)
- Large numbers for counts (Matched, Differed, Blocked)
- Timeline has visual flow with dots and lines

**Interaction:**
- View comparison outcome at a glance
- Scroll to see full replay lineage
- Understand observation envelope (what was captured)
- All information is read-only (no editing in Workbench mode)

---

#### 2. Evidence Bundle Panel (Left, Supporting)
**Traceable artifacts** - Source context + artifact list

**Contains:**
- **Source Formula (Compact):**
  - Formula code (10px monospace, very compact)
  - Result value (one-line summary)
  
- **Reliability Badge:**
  - Large percentage (98%)
  - Progress bar (visual confidence)
  - Description ("High confidence • 5 consistent runs")
  - Moss green gradient background
  
- **Evidence Artifacts:**
  - **Scenario:** Deterministic evaluation context (ID: scn-4f8e2a)
  - **Run:** Latest run #5, 1.2ms (ID: run-9a2c5f)
  - **Observation:** Captured state envelope (ID: obs-7d3e1b)
  - **Comparison:** Active comparison session (ID: cmp-2f9a6d)
  - **Witness:** 3 verification points (ID: wit-5c1e8a)
  - Each artifact has:
    - Icon (function-specific)
    - Name
    - Description
    - ID (traceable reference)
    
- **Bundle Status:**
  - "Bundle complete • Ready for handoff" (green checkmark)

**Visual priority:**
- Takes 25% of width
- Parchment background (#faf7f1)
- Cards for each artifact
- Reliability badge is prominent (gradient background)
- Comparison artifact has terracotta accent (current focus)

**Interaction:**
- View source formula (compact, read-only)
- See reliability at a glance
- Review artifact list (scenario, run, observation, comparison, witness)
- Bundle status indicates readiness for handoff

---

#### 3. Actions & Blocked Dimensions Panel (Right, Supporting)
**Next steps** - Primary/secondary actions, blocked dimensions, recommendations

**Contains:**
- **Primary Actions:**
  - **Handoff Evidence** (terracotta button): Transfer bundle for review
  - **Retain Bundle** (moss button): Archive for future reference
  
- **Secondary Actions:**
  - **Export as JSON** (white button): Download evidence bundle
  - **Widen Observation** (white button): Expand captured state
  
- **Blocked Dimensions:**
  - List of dimensions that cannot be compared
  - Each blocked item shows:
    - Lock icon
    - Dimension name
    - Reason (why blocked)
  - If none blocked: Info message ("No dimensions blocked")
  
- **Next Action Recommendation:**
  - Moss green gradient background
  - Explanation of current state
  - Recommended next step
  - Link to action ("Click 'Handoff Evidence' to proceed")

**Visual priority:**
- Takes 25% of width
- Actions are prominent (large buttons)
- Blocked dimensions (if any) use terracotta accent
- Recommendation has green background (positive guidance)

**Interaction:**
- Click primary actions (Handoff, Retain)
- Click secondary actions (Export, Widen)
- View blocked dimensions (if any)
- Read next action recommendation

---

### Nearby Secondary (Context Bar)

#### Workbench Context Bar
**Formula space level** - Scenario context, mode badge, evidence tools

**Left section:**
- Formula space title ("LET Formula Analysis")
- Dirty indicator (dot)
- **Scenario timestamp:** "Created: Apr 4, 2026 14:32" (read-only badge with calendar icon)

**Center:**
- **Mode badge:** "Workbench Mode" with briefcase icon (terracotta accent)

**Right section:**
- **Witness Chain button:** Opens witness chain drawer
- **Handoff History button:** Opens handoff history drawer
- Separator
- Run count (5 runs)
- Host profile (OC-H0)

**Interaction:**
- View scenario creation timestamp (read-only)
- Click Witness Chain → opens witness drawer
- Click Handoff History → opens handoff drawer
- See run count and host profile at a glance

---

### On-Demand Detail (Right Drawer)

#### Witness Chain Drawer
**Verification points** - Opens when you click "Witness Chain" in context bar

**Contains:**
- **Verification Points:** Timeline of witness events
  - **Initial Evaluation:** Formula parsed and evaluated successfully (Run #1)
  - **Replay Consistency:** Runs #2-4 matched initial result
  - **Latest Verification:** Run #5 confirmed consistency
  - Each point shows:
    - Checkmark/clock icon
    - Description
    - Run reference
    - Timestamp
  - Color-coded dots (teal for verified, amber for latest)
  - Connecting lines show progression
  
- **Witness Metadata:**
  - Total verifications count
  - Runs witnessed count
  - Witness ID (traceable reference)
  
- **Traceability Note:**
  - Explanation of witness chain purpose
  - How to use for verification

**Visual priority:**
- Slides in from right (37% width)
- Timeline layout (vertical)
- Teal/amber accents
- White background, organized sections

**Interaction:**
- Read through verification points from top to bottom
- See how witness chain validates consistency
- Click X to close

---

#### Handoff History Drawer
**Transfer tracking** - Opens when you click "Handoff History" in context bar

**Contains:**
- **Transfer History:**
  - If no handoffs: Info message ("No handoffs recorded yet")
  - If handoffs exist: Timeline of transfers
    - Each handoff shows:
      - Send icon
      - Recipient
      - Status (Pending/Completed)
      - Timestamp
      - Color-coded dot (terracotta)
  
- **Handoff Configuration:**
  - **Recipient** dropdown (Review Team, Archive, External Auditor)
  - **Include** checkboxes:
    - Evidence bundle (checked)
    - Replay lineage (checked)
    - Witness chain (checked)
    - Source formula (unchecked)
  - **Notes** textarea (for recipient message)
  
- **Handoff Action:**
  - "Create Handoff" button (terracotta, full width)

**Visual priority:**
- Slides in from right (37% width)
- Terracotta accents (handoff theme)
- Form layout for configuration
- Action button is prominent

**Interaction:**
- View handoff history (if any)
- Select recipient
- Choose what to include
- Add notes
- Click "Create Handoff" to transfer evidence

---

## Surface Placement Summary

| Surface | Always Visible | Nearby Secondary | On Demand |
|---------|----------------|------------------|-----------|
| **Comparison outcome** | ✓ Center top | | |
| **Replay lineage** | ✓ Center middle | | |
| **Observation envelope** | ✓ Center bottom | | |
| **Source formula** | ✓ Left top (compact) | | Edit in Explore mode |
| **Reliability badge** | ✓ Left (prominent) | | |
| **Evidence artifacts** | ✓ Left (list) | | |
| **Primary actions** | ✓ Right top | | Confirmation modals |
| **Secondary actions** | ✓ Right middle | | |
| **Blocked dimensions** | ✓ Right bottom | | |
| **Next action rec** | ✓ Right bottom | | |
| **Witness chain** | | Context bar button | Full drawer |
| **Handoff history** | | Context bar button | Full drawer |
| **Scenario timestamp** | | Context bar (read-only) | |

---

## Responsive Behavior

### Desktop (1920x1080 or wider)
**Ideal state** - All three columns visible

- Evidence Bundle: 25% (~480px)
- Main Workbench: 50% (~960px)
- Actions: 25% (~480px)
- Drawer replaces Actions when open

### Narrow Desktop (1366x768)
**Compressed but functional**

- Evidence Bundle: 25% (~341px)
- Main Workbench: 50% (~683px)
- Actions: 25% (~341px)
- Drawer replaces Actions when open
- May need to compress timeline cards

### Tablet/Browser (1024x768)
**Two-column priority**

- Evidence Bundle: Hidden, accessible via button
- Main Workbench: 70% (~716px)
- Actions: 30% (~308px)
- Drawer overlays when opened

### Minimum Width
**1280px recommended** for optimal experience

Below 1280px:
- Hide Evidence Bundle panel (show in modal on demand)
- Main Workbench: 65%
- Actions: 35%
- Keep comparison outcome and lineage visible

---

## Interaction Patterns

### Typical Workflow

1. **View comparison outcome** (matched, differed, blocked counts)
2. **Read outcome detail** (specific dimensions)
3. **Scroll through replay lineage** (timeline of runs)
4. **Check reliability badge** (confidence percentage)
5. **Review evidence artifacts** (scenario, run, observation, comparison, witness)
6. **Click Witness Chain** to see verification points
7. **Choose action:** Handoff, Retain, Export, or Widen
8. **Follow recommendation** for next step

### Comparing Runs

1. **Look at Matched count** in comparison outcome
2. **Check Differed count** for variance
3. **Review Blocked count** (if any)
4. **Read outcome detail** to see specific dimensions
5. **Scroll to replay lineage** to see timeline
6. **Verify latest run** (terracotta, pulsing)
7. **Understand consistency** across runs

### Viewing Witness Chain

1. **Click "Witness Chain"** button in context bar
2. **Drawer slides in** from right (Actions hides)
3. **Read verification points** from top to bottom
   - Initial Evaluation
   - Replay Consistency
   - Latest Verification
4. **See witness metadata** (total verifications, runs witnessed, ID)
5. **Understand traceability** (how witness validates evidence)
6. **Click X** to close drawer (Actions returns)

### Handing Off Evidence

1. **Check reliability badge** (ensure high confidence)
2. **Click "Handoff Evidence"** button in Actions panel
3. **Drawer slides in** with handoff configuration
4. **Select recipient** (Review Team, Archive, External Auditor)
5. **Choose what to include:**
   - Evidence bundle (checked by default)
   - Replay lineage (checked)
   - Witness chain (checked)
   - Source formula (optional)
6. **Add notes** for recipient
7. **Click "Create Handoff"** to transfer
8. **Handoff appears in history** (visible in Handoff History drawer)

### Retaining Evidence

1. **Review evidence bundle** (all artifacts present)
2. **Check reliability** (high confidence)
3. **Click "Retain Bundle"** button in Actions panel
4. **Confirmation modal** appears (optional)
5. **Evidence is archived** for future reference
6. **Bundle remains accessible** in workspace

### Exporting Evidence

1. **Click "Export as JSON"** button in Actions panel
2. **Download begins** immediately
3. **JSON file contains:**
   - Source formula
   - All runs (full timeline)
   - Comparison outcome
   - Observation envelope
   - Witness chain
   - Evidence artifact IDs
4. **Use for external analysis** or record-keeping

### Widening Observation

1. **Click "Widen Observation"** button in Actions panel
2. **Modal or drawer** shows options (future)
3. **Add dimensions** to capture:
   - Additional metadata
   - External context
   - User annotations
4. **Observation envelope updates** with new data
5. **Evidence bundle refreshes**

---

## Visual Identity

### Color Palette (Warm Editorial, Terracotta Accent)

**Background tones:**
- `#faf7f1` - Parchment (main background)
- `#f7f3ea` - Light parchment (headers, rails)
- `#ede7da` - Warm smoke (top bar, footer)
- `#ffffff` - White (workbench surface, cards)

**Primary accents (Workbench mode uses Terracotta):**
- `#b84532` - Terracotta (Workbench mode, primary actions, latest run, handoff)
- `#1e4d4a` - Oxidized teal (Matched state, verified items)
- `#c88d2e` - Amber brass (Differed state, witness items)
- `#3e5238` - Moss (Retain action, initial run, reliability)
- `#7a7568` - Warm gray (Blocked state, unavailable items)

**Text:**
- `#1f1c17` - Espresso ink (primary text)
- `#7a7568` - Warm gray (secondary text)

**Borders:**
- `#1f1c17/10` - 10% espresso (subtle borders)
- `#1f1c17/15` - 15% espresso (input borders)

### Comparison Outcome Colors

**Matched:**
- Background: `bg-gradient-to-br from-[#1e4d4a]/5 to-[#1e4d4a]/10`
- Border: `border-2 border-[#1e4d4a]/30`
- Text: `text-[#1e4d4a]`
- Icon: CheckCircle2

**Differed:**
- Background: `bg-gradient-to-br from-[#c88d2e]/5 to-[#c88d2e]/10`
- Border: `border-2 border-[#c88d2e]/30`
- Text: `text-[#c88d2e]`
- Icon: XCircle

**Blocked:**
- Background: `bg-gradient-to-br from-[#b84532]/5 to-[#b84532]/10`
- Border: `border-2 border-[#b84532]/30`
- Text: `text-[#b84532]`
- Icon: AlertTriangle

### Replay Lineage Colors

**Initial run:**
- Dot: `bg-[#3e5238]` (moss)
- Border: `border-[#3e5238]/20`

**Replay runs:**
- Dot: `bg-[#1e4d4a]` (teal)
- Border: `border-[#1e4d4a]/20`

**Latest run:**
- Dot: `bg-[#b84532]` (terracotta, pulsing)
- Border: `border-2 border-[#b84532]/30`
- Background: `bg-gradient-to-br from-[#b84532]/5 to-[#b84532]/10`

### Typography

**Headers:** System sans-serif, 600 weight, tight tracking
**Body:** System sans-serif, 400 weight, relaxed leading
**Monospace:** System monospace (Monaco, Menlo, Consolas)
- Source formula: 10px (extra compact)
- Result value: 36px (3xl)
- Metric counts: 36px (3xl)
- Artifact IDs: 10px (xs)

### Spacing

**Sections:**
- Section gaps: gap-6 (24px)
- Card padding: p-4 (16px)
- Button padding: px-4 py-3

**Timeline:**
- Dot: w-4 h-4 (16px)
- Line: w-0.5 (2px)
- Card padding: p-3 (12px)

### Borders & Shadows

**Borders:**
- Subtle: border border-[#1f1c17]/10
- Card: border border-[#1f1c17]/15
- Accent: border-2 border-[color]/30

**Shadows:**
- Drawer: shadow-2xl
- Buttons: shadow-sm
- Timeline dots: shadow-md

---

## Component Architecture

### FocusedWorkbenchMode (Shell)
**Root component** - Manages drawer state, layout orchestration

**Props:** None
**State:** 
- `drawerOpen` (null | 'witness' | 'handoff')

**Children:**
- Global top bar (DNA OneCalc branding)
- WorkspaceRail (left)
- WorkbenchContextBar (formula space level)
- EvidenceBundlePanel (left column)
- WorkbenchMainPanel (center column)
- WorkbenchActionsPanel (right column)
- WorkbenchDrawer (conditional, right drawer)
- Status footer

---

### WorkbenchContextBar
**Formula space level** - Scenario context, mode badge, evidence tools

**Props:** `onOpenDrawer(drawer: 'witness' | 'handoff')`
**State:** None

**Features:**
- Formula space title + dirty indicator
- Scenario timestamp (read-only badge)
- Mode badge (Workbench Mode with briefcase icon)
- Witness Chain button
- Handoff History button
- Run count + host profile

---

### EvidenceBundlePanel
**Traceable artifacts** - Source context + artifact list

**Props:** None
**State:** None

**Features:**
- Source formula (compact, read-only)
- Reliability badge (percentage, progress bar, description)
- Evidence artifacts list (scenario, run, observation, comparison, witness)
  - Each with icon, name, description, ID
- Bundle status ("Bundle complete • Ready for handoff")

**Interaction:**
- View source formula (compact)
- See reliability at a glance
- Review artifact list

---

### WorkbenchMainPanel
**The primary surface** - Comparison, lineage, observation

**Props:** None
**State:** None

**Features:**
- **Comparison Outcome:**
  - Three metric cards (Matched, Differed, Blocked)
  - Outcome detail list (specific dimensions)
  
- **Replay Lineage:**
  - Timeline of runs (vertical, color-coded)
  - Each run card (number, label, timestamp, result, time, policy)
  - Latest run highlighted (terracotta, pulsing)
  
- **Observation Envelope:**
  - Captured state grid (hash, profile, version, policy, timestamp, count)

**Interaction:**
- View comparison outcome
- Scroll through replay lineage
- Read observation envelope

---

### WorkbenchActionsPanel
**Next steps** - Primary/secondary actions, blocked dimensions, recommendations

**Props:** None
**State:** None

**Features:**
- Primary actions (Handoff Evidence, Retain Bundle)
- Secondary actions (Export as JSON, Widen Observation)
- Blocked dimensions list (if any)
- Next action recommendation (green gradient background)

**Interaction:**
- Click primary actions
- Click secondary actions
- View blocked dimensions
- Read recommendation

---

### WorkbenchDrawer
**Deep detail drawer** - Witness, Handoff

**Props:** 
- `type` (null | 'witness' | 'handoff')
- `onClose()`

**State:** None

**Features:**
- Drawer header (title, description, close button)
- Content area (WitnessContent or HandoffContent)
- Slides in from right, replaces Actions panel

**Behavior:**
- Renders only when `type` is not null
- Click X → calls `onClose()`

**Content Components:**

**WitnessContent:**
- Verification points (timeline)
- Witness metadata (counts, ID)
- Traceability note

**HandoffContent:**
- Transfer history (if any)
- Handoff configuration (recipient, include, notes)
- Create Handoff button

---

## State Management

### Formula-Space Level State
**Managed by FocusedWorkbenchMode**

- `drawerOpen` - Which drawer is open (null | 'witness' | 'handoff')
- `comparisonData` - Matched/Differed/Blocked counts and details
- `replayLineage` - Timeline of runs
- `observationEnvelope` - Captured state metadata
- `evidenceBundle` - Artifact list with IDs
- `reliabilityScore` - Percentage and description

### Component Local State

- **WorkbenchContextBar:** None (all interactions trigger drawer)
- **EvidenceBundlePanel:** None (displays static data)
- **WorkbenchMainPanel:** None (displays static data)
- **WorkbenchActionsPanel:** None (buttons trigger actions)

---

## Evidence Artifact Model

### Artifact Types

**Scenario:**
- **ID:** `scn-4f8e2a`
- **Type:** Evaluation context
- **Description:** Deterministic evaluation context
- **Icon:** FileText (moss green)

**Run:**
- **ID:** `run-9a2c5f`
- **Type:** Single evaluation execution
- **Description:** Evaluation #5 • 1.2ms
- **Icon:** Play (teal)

**Observation:**
- **ID:** `obs-7d3e1b`
- **Type:** Captured state envelope
- **Description:** Captured state envelope
- **Icon:** Eye (moss green)

**Comparison:**
- **ID:** `cmp-2f9a6d`
- **Type:** Comparison session
- **Description:** Active comparison session
- **Icon:** GitCompare (terracotta)

**Witness:**
- **ID:** `wit-5c1e8a`
- **Type:** Verification chain
- **Description:** 3 verification points
- **Icon:** Users (amber)

**Handoff (future):**
- **ID:** `hnd-8f4a3c`
- **Type:** Evidence transfer
- **Description:** Transferred to review team
- **Icon:** Send (terracotta)

### Artifact Relationships

```
Scenario (scn-4f8e2a)
  └── Run #1 (run-1a2b3c) [Initial]
      └── Observation (obs-7d3e1b)
  └── Run #2 (run-4d5e6f) [Replay]
  └── Run #3 (run-7g8h9i) [Replay]
  └── Run #4 (run-0j1k2l) [Replay]
  └── Run #5 (run-9a2c5f) [Latest]
      └── Comparison (cmp-2f9a6d)
          └── Witness (wit-5c1e8a)
              └── Handoff (future)
```

---

## Future Enhancements

### Comparison Enhancements
- **Diff view:** Side-by-side comparison of specific dimensions
- **Variance analysis:** Statistical analysis of timing variance
- **Threshold configuration:** Set acceptable variance thresholds

### Replay Improvements
- **Timeline scrubbing:** Click run to see detailed state
- **Run comparison:** Compare two specific runs
- **Replay automation:** Schedule automatic replays

### Evidence Management
- **Bundle versioning:** Track changes to evidence bundle
- **Annotation support:** Add notes to specific runs or artifacts
- **Search/filter:** Find runs by date, result, or timing

### Handoff Workflow
- **Review status:** Track handoff approval/rejection
- **Comment thread:** Discussion on handed-off evidence
- **Return flow:** Bring evidence back from review

### Blocked Dimensions
- **Detail view:** Deep dive into why dimension is blocked
- **Workaround suggestions:** How to unblock or approximate
- **External integration:** Show external API responses (if available)

### Widening Capabilities
- **Add dimensions:** Custom metadata fields
- **External context:** Link to external data sources
- **User annotations:** Rich text notes

---

## Design Rationale

### Why "Evidence" Language?

**Problem:** Terms like "debug log" or "run history" don't convey the serious, traceable nature of this work.

**Solution:** Use evidence language: **scenario, run, observation, comparison, witness, handoff**. These terms signal that this is not casual logging but serious evidence work.

**Rationale:** Formula evaluation in serious contexts (financial, regulatory, scientific) requires traceable evidence. The language should match the gravity.

### Why Reliability Badge is Prominent?

**Problem:** Without a clear reliability indicator, users don't know if they can trust the evidence.

**Solution:** **Large percentage + progress bar + description** at the top of Evidence Bundle panel. Green background, clear visibility.

**Rationale:** Reliability is the first question: "Can I trust this?" Answer it immediately.

### Why Replay Lineage is a Timeline?

**Problem:** A list of runs (Run #1, Run #2, etc.) doesn't show **progression** or **relationship** between runs.

**Solution:** **Visual timeline** with color-coded dots, connecting lines, and cards showing progression from initial to latest.

**Rationale:** Lineage implies ancestry. The timeline shows how runs are related over time, not just chronologically listed.

### Why Latest Run Pulses?

**Problem:** In a long timeline, it's hard to find the most recent run.

**Solution:** **Latest run has pulsing animation**, terracotta accent, and "LATEST" badge.

**Rationale:** Draw the eye to the current state. Pulsing suggests "active" or "live" state.

### Why Drawer Replaces Actions Panel?

**Problem:** If drawer overlays, user can't see workbench and drawer detail simultaneously. If we compress, everything is too narrow.

**Solution:** Drawer **replaces Actions panel**. Workbench and Evidence Bundle remain visible.

**Rationale:** Actions are useful at a glance, but when you need deep detail (witness chain, handoff config), you don't need the action buttons. Trade-off keeps workbench usable.

### Why "Blocked Dimensions" Instead of "Errors"?

**Problem:** "Errors" implies something went wrong. But some dimensions are inherently opaque (external APIs, black-box functions).

**Solution:** "**Blocked Dimensions**" - neutral term for things that can't be compared, with reasons.

**Rationale:** Transparency about limitations without blaming. Some things are just not observable, and that's okay—but we should be explicit about it.

---

## Key Differences from Explore and Inspect Modes

### Explore Mode (Formula Authoring)
- **Purpose:** Create and test formulas
- **Primary surface:** Formula editor (editable)
- **Result:** Large, prominent
- **Workflow:** Edit → Evaluate → See result → Adjust

### Inspect Mode (Semantic Understanding)
- **Purpose:** Understand how formula was evaluated
- **Primary surface:** Formula Walk (tree view)
- **Result:** Visible but secondary
- **Workflow:** View walk → Expand nodes → Trace provenance → Understand semantics

### Workbench Mode (Evidence & Comparison)
- **Purpose:** Build evidence, compare runs, handoff artifacts
- **Primary surface:** Main Workbench (comparison, lineage, observation)
- **Result:** Compact summary in Evidence Bundle
- **Workflow:** View comparison → Review lineage → Check reliability → Take action (handoff/retain/export)

**When to use Explore Mode:**
- You're writing a new formula
- You're testing different formulas
- You need to edit the formula

**When to use Inspect Mode:**
- You want to understand how a formula works
- You need to see intermediate values
- You're investigating why a result is unexpected

**When to use Workbench Mode:**
- You've run the formula multiple times
- You want to compare runs for consistency
- You need to build evidence for review/audit
- You're ready to handoff or archive evidence
- You want to understand reliability over time

---

## Conclusion

Focused Workbench Mode is an **evidence workbench** that prioritizes comparison, replay lineage, and traceable artifacts. The **comparison outcome** tells you what matched, what differed, and what's blocked. The **replay lineage** shows a visual timeline of runs from initial to latest. The **evidence bundle** tracks scenario, run, observation, comparison, and witness with unique IDs. Actions (export, retain, widen, handoff) are prominent, making it clear what you can *do* with the evidence.

**Core principle:** This is not a log viewer or admin page—it's a purpose-built workbench for serious formula evidence work. Everything is traceable, comparable, and ready for handoff.
