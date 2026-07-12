# DNA OneCalc Design Directions

## Overview
This application showcases three distinct design directions for DNA OneCalc, a serious single-formula calculation host. Each direction maintains the core product structure (Formula/Function Explorer → Live Formula Semantic X-Ray → Twin Oracle Workbench) while offering unique visual and interaction approaches.

## Color Palette
All three directions use a warm, editorial palette that avoids generic SaaS styling:

### Base Tones
- **Parchment**: `#f5f1e8` - Warm, approachable background
- **Sand**: `#e8dcc8` - Secondary surface tone
- **Smoke**: `#9a9a9a` - Muted text and borders
- **Ink**: `#2a2a2a` - Primary text

### Accent Families
- **Deep Teal**: `#2d5f5d` - Primary actions, success states
- **Terracotta**: `#c65d47` - Comparison, warnings
- **Amber**: `#d69f4c` - Highlights, numbers
- **Moss**: `#5a6f4d` - Supporting elements
- **Muted Rust**: `#a75842` - Evidence, emphasis

## Design Direction 1: Warm Editorial Rail-and-Tabs Workbench

### Key Characteristics
- **Visual Style**: Classic, approachable, warm
- **Layout**: Persistent left rail (240px) + horizontal tab strip
- **Navigation**: Familiar workspace patterns with clear hierarchy
- **Interaction**: Gentle transitions, comfortable density

### Best For
- Users transitioning from traditional spreadsheet tools
- Formula exploration and learning workflows
- Teams prioritizing approachability over density

### Unique Features
- Full-width left rail with clear space categorization
- Horizontal mode switcher (Explorer / X-Ray / Workbench)
- Collapsible X-Ray drawer on the right
- Warm, inviting color application

## Design Direction 2: Analytical Compare Studio

### Key Characteristics
- **Visual Style**: Evidence-first, structured, analytical
- **Layout**: Minimal left rail (64px icons) + integrated tab/mode bar
- **Navigation**: Comparison-optimized with side-by-side layouts
- **Interaction**: Dense information display, systematic presentation

### Best For
- Power users focusing on comparison and validation
- Evidence collection and replay workflows
- Scenarios requiring detailed diff analysis

### Unique Features
- Integrated tab and mode bar for compact navigation
- Side-by-side comparison grids (DNA vs Excel)
- Comparison dimension matrix with match badges
- Right-panel X-Ray with gradient header
- Evidence bundle and lineage tracking

## Design Direction 3: Modular Evidence Cockpit

### Key Characteristics
- **Visual Style**: Flexible, modular, customizable
- **Layout**: Collapsible rails + grid-based panel system
- **Navigation**: Panel-based with user control over layout
- **Interaction**: Dense, technical, optimized for multi-tasking

### Best For
- Advanced users with complex workflows
- Scenarios requiring multiple simultaneous views
- Users who prioritize customization and control

### Unique Features
- Dark mode with warm undertones
- 12-column grid system for panel arrangement
- User-configurable panel visibility
- Collapsible left and right rails
- Command bar and status bar
- Dense, technical information presentation
- Evidence workbench in right panel

## Common Elements Across All Directions

### Formula Editor
- Multiline editing with line numbers
- Syntax highlighting (functions, numbers, operators)
- Tab/Shift+Tab indentation support (implied)
- Keyboard-first interaction
- Not terminal-like, not full IDE

### X-Ray Inspection
- Parse context (status, tokens, functions)
- Bind context (variables, references)
- Eval context (steps, duration)
- Provenance (host profile, capability floor, platform)

### Twin Oracle Workbench
- DNA OneCalc vs Excel comparison
- Reliability badge and comparison envelope
- Evidence lineage tracking
- Scenario/run/evidence hierarchy
- Retain and handoff actions

### Multi-Space Support
- Active/dirty/retained/blocked states
- Workspace rail and tab strip
- Per-space context preservation
- Clear visual indicators for space state

## Technical Implementation

### Stack
- React 18.3.1
- React Router 7 (Data mode)
- Tailwind CSS 4.1
- TypeScript
- Lucide React for icons

### Responsive Behavior
- Desktop-first but browser-capable
- Formula editor and result area prioritized on narrow layouts
- Supporting surfaces collapse or stack before primary surfaces sacrifice space
- Minimum viable width maintains editor and result visibility

### State Management
- Local component state for UI interactions
- Per-direction view mode management
- Panel visibility configuration (Modular direction)
- Rail collapse/expand state

## Design Rationale

### Why Three Directions?
1. **Different User Modes**: Explorer vs Investigator vs Operator require different optimizations
2. **Workflow Variations**: Some users prioritize approachability, others density
3. **Evolution Path**: Shows multiple valid futures for the product

### Why This Palette?
- Avoids purple-on-white SaaS clichés
- Warm tones feel editorial and intentional
- Sufficient contrast for technical content
- Distinctive and memorable
- Works across all three directions

### Why Rail + Tabs?
- Compact multi-space affordance
- Shows identity, state, and grouping
- Keeps active scenario obvious
- Browser-implementable (no complex window management)
- Scales better than split panes for primary workflow

## Future Considerations

### Not Yet Implemented
- Split pane views for comparison workflows
- Secondary window support for advanced monitoring
- Full capability center UI
- Extension state visualization
- Advanced layout customization in non-modular directions

### Intentionally Deferred
- Actual formula evaluation
- Real OxFml/OxFunc integration
- Live Excel observation via OxXlPlay
- Persistence and document model
- Scenario capsule import/export

## Usage

Navigate between design directions using the landing page, or directly:
- `/` - Design direction selector
- `/warm-editorial` - Direction 1
- `/analytical-compare` - Direction 2
- `/modular-evidence` - Direction 3

Each direction shows:
1. Full app shell with navigation
2. Formula editor with syntax highlighting
3. Result display
4. X-Ray drawer/panel
5. Compare/workbench view
6. Multi-space rail and tabs
