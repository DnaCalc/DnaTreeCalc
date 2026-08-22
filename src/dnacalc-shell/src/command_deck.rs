//! Command deck v1 — SHELL_SPEC.md §5.1/§7, MECHANISMS 08.
//!
//! A one-at-a-time overlay (Ctrl+K primary, Ctrl+Shift+P mirror; Ctrl+F opens
//! it in goto mode) that lets a user find and run any command by name, and
//! jump the selection by name / sheet / node-path / A1 address.
//!
//! Three command sources (SHELL_SPEC §5.1 scope):
//! 1. **Catalog commands** — adapted in-process from
//!    [`WorkspaceState::command_catalog`] governed by the active persona.
//!    v1 surfaces the catalog's *no-target* commands (recalc / undo / redo /
//!    new-workspace): the catalog never synthesizes an intent payload
//!    (layering law), so target-constructing commands (edit, add, copy, …)
//!    stay with their stage affordances and are not run from the deck.
//!    A serializable catalog is gap **G8** — this adapter is in-process only;
//!    it invents no wire format.
//! 2. **Shell-control commands** — stage switch, registry/inspector toggle,
//!    theme/density switch, atlas/timeline open, persona switch, and
//!    document-lifecycle save/open. Save/Open enablement tracks the host's
//!    advertised `HostCapabilityProjection`/`PersistenceProjection`
//!    capability (bead dtc-lfz.3): honestly disabled-with-reason when the
//!    host has not wired persistence, and a real `SkinShellIntent` dispatch
//!    over the Shell's `on_shell_intent` channel when it has.
//! 3. **Goto** — defined names + sheet names + node display paths matched as
//!    strings against [`WorkspaceState`], plus A1-style refs recognized as
//!    **navigation sugar only** ([`parse_a1`]): a pure address *grammar*, not
//!    formula parsing — the deck never tokenizes or evaluates anything, it
//!    only recognizes the address shape, canonicalizes it, and dispatches a
//!    `SelectNode` exactly as the Name-Box does with a typed string.
//!
//! Every entry carries a **stable id string** ([`DeckCommand::id`]). Ids are
//! agent API (mechanism 20) — [`static_command_ids`] is snapshot-tested and
//! a rename is a breaking change.

use dnacalc_skin_ir::command::{CommandCatalogProjection, CommandIntentKindProjection};
use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::WorkspaceIntent;
use dnacalc_skin_ir::keychord::SkinVerb;
use dnacalc_skin_ir::permissions::Persona;
use dnacalc_skin_ir::protocol::SkinShellIntent;
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_strand::{Density, Theme};

use crate::keyboard::{ShellKeyboardRegistry, chord_display};

/// Which grouped section a command belongs to in the deck listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckGroup {
    /// Goto results (dynamic; query-derived). Shown first.
    Goto,
    /// Catalog commands (recalc / undo / redo / new-workspace).
    Command,
    /// Stage switching.
    Stage,
    /// View-state: registry/inspector toggles, theme, density, overlays.
    View,
    /// Persona switching.
    Persona,
    /// Document lifecycle (save / open).
    Document,
}

impl DeckGroup {
    /// Display order.
    #[must_use]
    pub const fn order(self) -> u8 {
        match self {
            Self::Goto => 0,
            Self::Command => 1,
            Self::Stage => 2,
            Self::View => 3,
            Self::Persona => 4,
            Self::Document => 5,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Goto => "Go to",
            Self::Command => "Commands",
            Self::Stage => "Stages",
            Self::View => "View",
            Self::Persona => "Persona",
            Self::Document => "Document",
        }
    }
}

/// What executing a deck entry does. Never executed while
/// [`DeckCommand::enabled`] is false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckAction {
    /// Dispatch a workspace intent through the one dispatcher (recalc, undo,
    /// redo, new-workspace, persona switch, and every goto selection).
    Dispatch(WorkspaceIntent),
    /// Switch to the visible stage in this 1-based slot (re-projection).
    SwitchStage(u8),
    ToggleRegistry,
    ToggleInspector,
    OpenAtlas,
    OpenTimeline,
    SetTheme(Theme),
    SetDensity(Density),
    /// Dispatch a `SkinShellIntent` through `OverlayContext::shell_intent`
    /// (SHELL_SPEC §4, bead dtc-lfz.3) — Save/Open document lifecycle. Built
    /// unconditionally on the entry regardless of `enabled`; `run_action` is
    /// never called for a disabled command, so this only ever fires when the
    /// host has actually advertised the capability.
    EmitShellIntent(SkinShellIntent),
    /// Present for discoverability but not runnable here (host support
    /// absent); the disabled reason explains why.
    Inert,
}

/// One deck entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckCommand {
    /// Stable id — agent API. Catalog ids are the projection's `stable_id`
    /// (`recalculate`, `undo`, …); shell ids are `shell.*`; goto ids are
    /// `goto:*` (dynamic, not in the static snapshot set).
    pub id: String,
    pub title: String,
    pub group: DeckGroup,
    /// Effective chord from the live keyboard registry, if the command has
    /// one.
    pub chord: Option<String>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub action: DeckAction,
}

/// The read-only inputs the deck builds its command list from — a plain
/// snapshot so the whole build + filter + goto grammar is unit-testable
/// without Leptos or a browser.
pub struct DeckInputs<'a> {
    pub workspace: &'a WorkspaceState,
    pub selection: &'a SelectionState,
    pub persona: Persona,
    /// Visible stages in slot order: (1-based slot, title).
    pub visible_stages: Vec<(u8, &'static str)>,
    pub theme: Theme,
    pub density: Density,
    pub registry: &'a ShellKeyboardRegistry,
    /// Host capability: is saving wired yet? (false in S0 → disabled badge.)
    pub host_can_save: bool,
    pub host_can_open: bool,
}

impl DeckInputs<'_> {
    fn chord_for(&self, verb: SkinVerb) -> Option<String> {
        self.registry.primary_chord(verb).map(chord_display)
    }
}

/// The catalog intent kinds the deck runs directly (no caller-constructed
/// payload). Order here is the display order within [`DeckGroup::Command`].
const RUNNABLE_CATALOG_KINDS: [CommandIntentKindProjection; 4] = [
    CommandIntentKindProjection::Recalculate,
    CommandIntentKindProjection::Undo,
    CommandIntentKindProjection::Redo,
    CommandIntentKindProjection::NewWorkspace,
];

/// The zero-payload intent for a runnable catalog kind.
fn runnable_intent(kind: CommandIntentKindProjection) -> WorkspaceIntent {
    match kind {
        CommandIntentKindProjection::Recalculate => WorkspaceIntent::Recalculate,
        CommandIntentKindProjection::Undo => WorkspaceIntent::Undo,
        CommandIntentKindProjection::Redo => WorkspaceIntent::Redo,
        CommandIntentKindProjection::NewWorkspace => WorkspaceIntent::NewWorkspace,
        // Only the four runnable kinds ever reach here.
        other => unreachable!("non-runnable catalog kind {other:?} routed to runnable_intent"),
    }
}

fn catalog_verb(kind: CommandIntentKindProjection) -> Option<SkinVerb> {
    match kind {
        CommandIntentKindProjection::Recalculate => Some(SkinVerb::Recalculate),
        CommandIntentKindProjection::Undo => Some(SkinVerb::Undo),
        CommandIntentKindProjection::Redo => Some(SkinVerb::Redo),
        CommandIntentKindProjection::NewWorkspace => Some(SkinVerb::NewWorkspace),
        _ => None,
    }
}

/// Build the static (non-goto) command list from `inputs`. Deterministic and
/// snapshot-pinned via [`static_command_ids`].
#[must_use]
pub fn static_commands(inputs: &DeckInputs) -> Vec<DeckCommand> {
    let mut commands = Vec::new();

    // --- (1) Catalog commands: in-process adapter, persona-governed ---
    let catalog: CommandCatalogProjection = inputs
        .workspace
        .command_catalog(inputs.selection)
        .governed_by(inputs.persona);
    for kind in RUNNABLE_CATALOG_KINDS {
        let Some(meta) = catalog.get(kind) else {
            continue;
        };
        commands.push(DeckCommand {
            id: kind.stable_id().to_string(),
            title: meta.title.to_string(),
            group: DeckGroup::Command,
            chord: catalog_verb(kind).and_then(|verb| inputs.chord_for(verb)),
            enabled: meta.enabled,
            disabled_reason: meta.disabled_reason.clone(),
            action: DeckAction::Dispatch(runnable_intent(kind)),
        });
    }

    // --- (2) Shell-control commands ---
    // Stage switching.
    for (slot, title) in &inputs.visible_stages {
        commands.push(DeckCommand {
            id: format!("shell.stage.{slot}"),
            title: format!("Go to stage: {title}"),
            group: DeckGroup::Stage,
            chord: inputs.chord_for(SkinVerb::SwitchLens(*slot)),
            enabled: true,
            disabled_reason: None,
            action: DeckAction::SwitchStage(*slot),
        });
    }

    // View toggles + overlays.
    commands.push(DeckCommand {
        id: "shell.registry.toggle".to_string(),
        title: "Toggle registry rail".to_string(),
        group: DeckGroup::View,
        chord: inputs.chord_for(SkinVerb::RegistryToggle),
        enabled: true,
        disabled_reason: None,
        action: DeckAction::ToggleRegistry,
    });
    commands.push(DeckCommand {
        id: "shell.inspector.toggle".to_string(),
        title: "Toggle inspector".to_string(),
        group: DeckGroup::View,
        chord: inputs.chord_for(SkinVerb::InspectorToggle),
        enabled: true,
        disabled_reason: None,
        action: DeckAction::ToggleInspector,
    });
    commands.push(DeckCommand {
        id: "shell.atlas.open".to_string(),
        title: "Open keyboard atlas".to_string(),
        group: DeckGroup::View,
        chord: inputs.chord_for(SkinVerb::KeyboardAtlas),
        enabled: true,
        disabled_reason: None,
        action: DeckAction::OpenAtlas,
    });
    commands.push(DeckCommand {
        id: "shell.timeline.open".to_string(),
        title: "Open timeline".to_string(),
        group: DeckGroup::View,
        chord: inputs.chord_for(SkinVerb::Timeline),
        enabled: true,
        disabled_reason: None,
        action: DeckAction::OpenTimeline,
    });

    // Theme switch — one entry per theme, the active one marked.
    for theme in Theme::ALL {
        let active = theme == inputs.theme;
        commands.push(DeckCommand {
            id: format!("shell.theme.{}", theme.id()),
            title: format!(
                "Theme: {}{}",
                theme_label(theme),
                if active { " (active)" } else { "" }
            ),
            group: DeckGroup::View,
            chord: None,
            enabled: !active,
            disabled_reason: active.then(|| "already the active theme".to_string()),
            action: DeckAction::SetTheme(theme),
        });
    }

    // Density switch.
    for density in Density::ALL {
        let active = density == inputs.density;
        commands.push(DeckCommand {
            id: format!("shell.density.{}", density.id()),
            title: format!(
                "Density: {}{}",
                density_label(density),
                if active { " (active)" } else { "" }
            ),
            group: DeckGroup::View,
            chord: None,
            enabled: !active,
            disabled_reason: active.then(|| "already the active density".to_string()),
            action: DeckAction::SetDensity(density),
        });
    }

    // --- Persona switch (dispatched SetPersona; audited) ---
    for persona in [Persona::Author, Persona::Reviewer, Persona::ReadOnly] {
        let active = persona == inputs.persona;
        commands.push(DeckCommand {
            id: format!("shell.persona.{}", persona.stable_id()),
            title: format!(
                "Persona: {}{}",
                persona_label(persona),
                if active { " (active)" } else { "" }
            ),
            group: DeckGroup::Persona,
            chord: None,
            enabled: !active,
            disabled_reason: active.then(|| "already the active persona".to_string()),
            action: DeckAction::Dispatch(WorkspaceIntent::SetPersona { persona }),
        });
    }

    // --- Document lifecycle: real capability, honest disabled-with-reason
    // otherwise (bead dtc-lfz.3). `enabled` tracks the host's *advertised*
    // `host_can_save`/`host_can_open` (threaded from the product's own
    // `HostCapabilityProjection`/`PersistenceProjection` through the Shell's
    // `host_persistence` prop); the action always carries a real
    // `SkinShellIntent` so a capable host genuinely saves/opens the instant
    // it advertises `true` — never a fabricated success.
    commands.push(DeckCommand {
        id: "shell.save".to_string(),
        title: "Save".to_string(),
        group: DeckGroup::Document,
        chord: inputs.chord_for(SkinVerb::Save),
        enabled: inputs.host_can_save,
        disabled_reason: (!inputs.host_can_save)
            .then(|| "the host does not yet support saving".to_string()),
        action: DeckAction::EmitShellIntent(SkinShellIntent::Save),
    });
    commands.push(DeckCommand {
        id: "shell.open".to_string(),
        title: "Open".to_string(),
        group: DeckGroup::Document,
        chord: inputs.chord_for(SkinVerb::Open),
        enabled: inputs.host_can_open,
        disabled_reason: (!inputs.host_can_open)
            .then(|| "the host does not yet support opening".to_string()),
        // No path picker exists at the deck (or in the Bench product at all)
        // yet, so this always dispatches with no requested path — a capable
        // host either resolves a default/current location or returns a
        // typed rejection asking for one; it is a real dispatch either way,
        // never a silent no-op.
        action: DeckAction::EmitShellIntent(SkinShellIntent::Open {
            requested_path: None,
        }),
    });

    commands
}

/// The stable id list for a given input snapshot — the agent-API surface,
/// snapshot-tested. Goto ids are dynamic and excluded by construction.
#[must_use]
pub fn static_command_ids(inputs: &DeckInputs) -> Vec<String> {
    static_commands(inputs)
        .into_iter()
        .map(|command| command.id)
        .collect()
}

// --- Goto ------------------------------------------------------------------

/// A parsed A1-style address. **Not** a formula parse: this is a pure address
/// grammar (optional `Sheet!` prefix, column letters, row digits) used only
/// to canonicalize a navigation target string. The deck never evaluates it,
/// never builds an expression, and never inspects a leading `=` — an A1 goto
/// dispatches a `SelectNode` with the canonical address string exactly as the
/// Name-Box dispatches a selection for a typed name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A1Address {
    pub sheet: Option<String>,
    /// 1-based column (A = 1).
    pub col: u32,
    /// 1-based row.
    pub row: u32,
}

impl A1Address {
    /// The canonical `Sheet!C4` / `C4` display + selection-target string.
    #[must_use]
    pub fn canonical(&self) -> String {
        let cell = format!("{}{}", column_letters(self.col), self.row);
        match &self.sheet {
            Some(sheet) => format!("{sheet}!{cell}"),
            None => cell,
        }
    }
}

fn column_letters(mut col: u32) -> String {
    let mut letters = Vec::new();
    while col > 0 {
        let rem = (col - 1) % 26;
        letters.push((b'A' + rem as u8) as char);
        col = (col - 1) / 26;
    }
    letters.iter().rev().collect()
}

/// Recognize an A1-style address (pure grammar; see [`A1Address`]). Returns
/// `None` for anything that is not exactly `[Sheet!]<letters><digits>` —
/// plain names and dotted node paths fall through to string-match goto.
#[must_use]
pub fn parse_a1(input: &str) -> Option<A1Address> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (sheet, cell) = match trimmed.split_once('!') {
        Some((sheet, cell)) => {
            if sheet.is_empty() || cell.is_empty() {
                return None;
            }
            (Some(sheet.to_string()), cell)
        }
        None => (None, trimmed),
    };

    let letters: String = cell
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let digits: String = cell.chars().skip(letters.len()).collect();
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    // Column letters → 1-based index (bijective base-26). Guard overflow so a
    // pathological "ZZZZZZ99999" can never panic.
    let mut col: u32 = 0;
    for ch in letters.chars() {
        let value = ch.to_ascii_uppercase() as u32 - 'A' as u32 + 1;
        col = col.checked_mul(26)?.checked_add(value)?;
    }
    let row: u32 = digits.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some(A1Address { sheet, col, row })
}

/// Cap on goto matches surfaced per source, so a huge workspace cannot flood
/// the deck.
const GOTO_MATCH_CAP: usize = 20;

/// Build the query-derived goto entries: A1 address (navigation sugar) first,
/// then sheet names, defined names, and node display paths that string-match
/// `query` (case-insensitive substring). Empty query → no goto entries (the
/// deck shows commands until the user starts a goto).
#[must_use]
pub fn goto_commands(inputs: &DeckInputs, query: &str) -> Vec<DeckCommand> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let mut commands = Vec::new();

    // A1 address grammar — navigation sugar only.
    if let Some(address) = parse_a1(query) {
        let canonical = address.canonical();
        commands.push(DeckCommand {
            id: format!("goto:a1:{canonical}"),
            title: format!("Go to cell {canonical}"),
            group: DeckGroup::Goto,
            chord: None,
            enabled: true,
            disabled_reason: None,
            action: DeckAction::Dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(canonical)))),
        });
    }

    let needle = query.to_lowercase();
    let contains = |haystack: &str| haystack.to_lowercase().contains(&needle);

    // Sheet names → select the sheet's grid node (a real NodeId).
    for sheet in inputs
        .workspace
        .sheets
        .iter()
        .filter(|s| contains(&s.display_name))
    {
        commands.push(DeckCommand {
            id: format!("goto:sheet:{}", sheet.grid_node_id),
            title: format!("Go to sheet: {}", sheet.display_name),
            group: DeckGroup::Goto,
            chord: None,
            enabled: true,
            disabled_reason: None,
            action: DeckAction::Dispatch(WorkspaceIntent::SelectNode(Some(
                sheet.grid_node_id.clone(),
            ))),
        });
        if commands.len() >= GOTO_MATCH_CAP {
            return commands;
        }
    }

    // Defined names → SelectNode by name string, mirroring Name-Box: the host
    // resolves the name to its node; an unknown name selects nothing (a
    // faithful no-op), never a fabricated node.
    for name in inputs
        .workspace
        .defined_names
        .entries
        .iter()
        .filter(|entry| contains(&entry.name))
    {
        commands.push(DeckCommand {
            id: format!("goto:name:{}", name.name),
            title: format!("Go to name: {}", name.name),
            group: DeckGroup::Goto,
            chord: None,
            enabled: true,
            disabled_reason: None,
            action: DeckAction::Dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(
                name.name.clone(),
            )))),
        });
        if commands.len() >= GOTO_MATCH_CAP {
            return commands;
        }
    }

    // Node display paths → select the real NodeId. Match against both the
    // dotted path (the NodeId) and the human display name.
    for node_id in &inputs.workspace.node_order {
        let display = inputs
            .workspace
            .node(node_id)
            .map(|node| node.display_name.as_str())
            .unwrap_or_default();
        if contains(node_id.as_str()) || contains(display) {
            commands.push(DeckCommand {
                id: format!("goto:node:{node_id}"),
                title: format!("Go to node: {node_id}"),
                group: DeckGroup::Goto,
                chord: None,
                enabled: true,
                disabled_reason: None,
                action: DeckAction::Dispatch(WorkspaceIntent::SelectNode(Some(node_id.clone()))),
            });
            if commands.len() >= GOTO_MATCH_CAP {
                return commands;
            }
        }
    }

    commands
}

// --- Fuzzy filter ----------------------------------------------------------

/// A deterministic subsequence fuzzy score for `title` against `query`
/// (both compared case-insensitively). `None` when `query` is not a
/// subsequence of `title`. Higher is better. An empty query scores every
/// title equally (`0`), preserving the caller's order.
#[must_use]
pub fn fuzzy_score(query: &str, title: &str) -> Option<i32> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(0);
    }
    let title_lower = title.to_lowercase();
    let title_bytes: Vec<char> = title_lower.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    let mut score: i32 = 0;
    let mut title_index = 0usize;
    let mut last_match: Option<usize> = None;
    for &qc in &query_chars {
        let mut found = None;
        while title_index < title_bytes.len() {
            if title_bytes[title_index] == qc {
                found = Some(title_index);
                break;
            }
            title_index += 1;
        }
        let position = found?;
        // Reward matches at the very start and matches contiguous with the
        // previous one; penalize the gap otherwise.
        if position == 0 {
            score += 12;
        }
        match last_match {
            Some(previous) if position == previous + 1 => score += 8,
            Some(previous) => score -= (position - previous - 1).min(6) as i32,
            None => score -= position.min(6) as i32,
        }
        last_match = Some(position);
        title_index += 1;
    }
    Some(score)
}

/// Filter and rank `commands` against `query`. Deterministic: sorted by score
/// descending, ties broken by the command's original index (stable), so the
/// same inputs always yield the same order. Disabled commands are kept
/// (shown with their reason) but rank below equal-scored enabled ones.
#[must_use]
pub fn filter_commands(query: &str, commands: Vec<DeckCommand>) -> Vec<DeckCommand> {
    let mut scored: Vec<(usize, i32, DeckCommand)> = commands
        .into_iter()
        .enumerate()
        .filter_map(|(index, command)| {
            fuzzy_score(query, &command.title).map(|score| (index, score, command))
        })
        .collect();
    scored.sort_by(|a, b| {
        // Higher score first; enabled before disabled at equal score; then
        // original index for a total, stable order.
        b.1.cmp(&a.1)
            .then_with(|| b.2.enabled.cmp(&a.2.enabled))
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.into_iter().map(|(_, _, command)| command).collect()
}

/// The full deck listing for a query: goto entries first (query-derived),
/// then filtered+ranked static commands.
#[must_use]
pub fn deck_listing(inputs: &DeckInputs, query: &str) -> Vec<DeckCommand> {
    let mut listing = goto_commands(inputs, query);
    listing.extend(filter_commands(query, static_commands(inputs)));
    listing
}

fn theme_label(theme: Theme) -> &'static str {
    match theme {
        Theme::CockpitLight => "Cockpit Light",
        Theme::CockpitDark => "Cockpit Dark",
        Theme::HighContrast => "High Contrast",
    }
}

fn density_label(density: Density) -> &'static str {
    match density {
        Density::Working => "Working",
        Density::Reading => "Reading",
    }
}

fn persona_label(persona: Persona) -> &'static str {
    match persona {
        Persona::Author => "Author",
        Persona::Reviewer => "Reviewer",
        Persona::ReadOnly => "Read-only",
    }
}

// --- Leptos component + OverlaySurface --------------------------------------

use leptos::prelude::*;

use crate::overlay::{OverlayContext, OverlaySurface};

/// The default command deck, mounted through
/// [`ShellOverlaySlots::command_deck`](crate::ShellOverlaySlots). A product
/// may substitute its own [`OverlaySurface`]; the shell installs this one when
/// the slot is empty.
#[derive(Default)]
pub struct CommandDeck;

impl OverlaySurface for CommandDeck {
    fn mount(&self, ctx: OverlayContext) -> AnyView {
        command_deck_view(ctx)
    }
}

/// Execute one deck action against the overlay context. Never called for a
/// disabled command.
fn run_action(ctx: &OverlayContext, action: &DeckAction) {
    match action {
        DeckAction::Dispatch(intent) => {
            ctx.dispatch.dispatch(intent.clone());
        }
        DeckAction::SwitchStage(slot) => ctx.controls.switch_stage.run(*slot),
        DeckAction::ToggleRegistry => ctx.controls.toggle_registry.run(()),
        DeckAction::ToggleInspector => ctx.controls.toggle_inspector.run(()),
        DeckAction::OpenAtlas => ctx.controls.open_atlas.run(()),
        DeckAction::OpenTimeline => ctx.controls.open_timeline.run(()),
        DeckAction::SetTheme(theme) => ctx.controls.set_theme.run(*theme),
        DeckAction::SetDensity(density) => ctx.controls.set_density.run(*density),
        DeckAction::EmitShellIntent(intent) => {
            // `None` here means the product has not wired the shell-intent
            // channel (`Shell`'s `on_shell_intent` prop) — never reached in
            // practice since that also means `host_can_save`/`host_can_open`
            // are `false`, so the entry is disabled and `run_action` is never
            // called for it. No-op rather than panic if misused regardless.
            if let Some(callback) = ctx.shell_intent {
                callback.run(intent.clone());
            }
        }
        // Inert commands are always disabled, so this arm is unreachable in
        // practice; do nothing rather than panic if a caller misuses it.
        DeckAction::Inert => {}
    }
}

fn command_deck_view(ctx: OverlayContext) -> AnyView {
    let query = RwSignal::new(String::new());

    // The reactive listing: rebuilt from live workspace/selection/persona/
    // overrides and the current query. The in-process catalog adapter lives
    // right here (`WorkspaceState::command_catalog`), governed by the shared
    // persona — the deck invents no wire format (G8).
    let ctx_for_list = ctx.clone();
    let listing = Memo::new(move |_| {
        let query_text = query.get();
        ctx_for_list.workspace.with(|workspace| {
            ctx_for_list.selection.with(|selection| {
                let persona = ctx_for_list.shared.with(|state| state.persona);
                let inputs = DeckInputs {
                    workspace,
                    selection,
                    persona,
                    visible_stages: ctx_for_list.visible_stages.clone(),
                    theme: ctx_for_list.theme,
                    density: ctx_for_list.density,
                    registry: &ctx_for_list.keyboard,
                    host_can_save: ctx_for_list.host_can_save,
                    host_can_open: ctx_for_list.host_can_open,
                };
                deck_listing(&inputs, &query_text)
            })
        })
    });

    let ctx_for_enter = ctx.clone();
    let execute_first = move || {
        let first_enabled =
            listing.with(|items| items.iter().find(|command| command.enabled).cloned());
        if let Some(command) = first_enabled {
            run_action(&ctx_for_enter, &command.action);
            ctx_for_enter.controls.close.run(());
        }
    };

    let placeholder = if ctx.goto_mode {
        "Go to a name, sheet, node path, or cell (A1)…"
    } else {
        "Type a command or go to…"
    };

    // The row-render context: Send + Sync + Clone, so each row's click handler
    // gets its own clone without an Rc (which is not Send, as Leptos render
    // closures require).
    let ctx_for_rows = ctx.clone();
    // A dedicated clone for the input's Escape handler (bead dtc-1tk.1 /
    // H1a) — separate from `ctx_for_enter`, which `execute_first` already
    // owns.
    let ctx_for_escape = ctx.clone();

    view! {
        <div class="dna-overlay-backdrop">
            <div
                class="dna-overlay dna-deck"
                role="dialog"
                aria-label="Command deck"
                data-overlay="command-deck"
                data-goto-mode=if ctx.goto_mode { "true" } else { "false" }
            >
                <input
                    class="dna-deck__input"
                    type="text"
                    autofocus
                    placeholder=placeholder
                    aria-label="Command deck query"
                    prop:value=move || query.get()
                    on:input=move |ev| query.set(event_target_value(&ev))
                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                        match ev.key().as_str() {
                            "Enter" => {
                                ev.prevent_default();
                                execute_first();
                            }
                            "Escape" => {
                                // The deck owns a pure-cancel Esc affordance: this
                                // input is autofocused the instant the deck opens,
                                // so a real Escape keydown targets it directly and
                                // never reaches the shell's own keydown handler
                                // (`event_target_is_text_entry` trips the §5
                                // text-entry guard for a bare, unmodified Escape —
                                // see shell.rs's `route_key` call). Closing here,
                                // deck-local, runs NO command (unlike Enter) and
                                // stops propagation so the shell's guard is never
                                // even consulted for this keystroke.
                                ev.prevent_default();
                                ev.stop_propagation();
                                ctx_for_escape.controls.close.run(());
                            }
                            _ => {}
                        }
                    }
                />
                <ul class="dna-deck__list" role="listbox">
                    {move || {
                        let ctx_rows = ctx_for_rows.clone();
                        listing
                            .get()
                            .into_iter()
                            .map(move |command| {
                                let command_for_click = command.clone();
                                let ctx_click = ctx_rows.clone();
                                view! {
                                    <li
                                        class="dna-deck__row"
                                        class:dna-deck__row--disabled=!command.enabled
                                        role="option"
                                        data-command-id=command.id.clone()
                                        data-enabled=if command.enabled { "true" } else { "false" }
                                        aria-disabled=if command.enabled { "false" } else { "true" }
                                        title=command.disabled_reason.clone().unwrap_or_default()
                                        on:click=move |_| {
                                            if command_for_click.enabled {
                                                run_action(&ctx_click, &command_for_click.action);
                                                ctx_click.controls.close.run(());
                                            }
                                        }
                                    >
                                        <span class="dna-deck__group" data-group=group_id(command.group)>
                                            {command.group.label()}
                                        </span>
                                        <span class="dna-deck__title">{command.title.clone()}</span>
                                        {command
                                            .disabled_reason
                                            .clone()
                                            .map(|reason| {
                                                view! {
                                                    <span class="dna-deck__reason">{reason}</span>
                                                }
                                            })}
                                        <span class="dna-deck__spacer"></span>
                                        {command
                                            .chord
                                            .clone()
                                            .map(|chord| {
                                                view! { <span class="dna-deck__chord">{chord}</span> }
                                            })}
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </div>
        </div>
    }
    .into_any()
}

fn group_id(group: DeckGroup) -> &'static str {
    match group {
        DeckGroup::Goto => "goto",
        DeckGroup::Command => "command",
        DeckGroup::Stage => "stage",
        DeckGroup::View => "view",
        DeckGroup::Persona => "persona",
        DeckGroup::Document => "document",
    }
}

/// The deck's component-scoped styles (resolve through Strand `--dna-*`).
pub const COMMAND_DECK_CSS: &str = r#"
.dna-deck { min-width: min(520px, calc(100vw - 24px)); max-width: calc(100vw - 24px); padding: 0; overflow: hidden; }
.dna-deck__input {
  width: 100%;
  box-sizing: border-box;
  border: none;
  border-bottom: 1px solid var(--dna-line);
  padding: var(--dna-gap-4) var(--dna-gap-5);
  font: inherit;
  font-size: 15px;
  background: var(--dna-paper);
  color: var(--dna-ink);
  outline: none;
}
.dna-deck__list { list-style: none; margin: 0; padding: var(--dna-gap-2); max-height: 60vh; overflow-y: auto; }
.dna-deck__row {
  display: flex;
  align-items: center;
  gap: var(--dna-gap-3);
  padding: var(--dna-gap-3) var(--dna-gap-4);
  border-radius: var(--dna-radius-chip);
  cursor: pointer;
  font-size: 13px;
}
.dna-deck__row:hover { background: var(--dna-accent-soft); }
.dna-deck__row--disabled { cursor: not-allowed; opacity: 0.55; }
.dna-deck__row--disabled:hover { background: transparent; }
.dna-deck__group {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--dna-ink-3);
  min-width: 64px;
}
.dna-deck__title { color: var(--dna-ink); }
.dna-deck__reason { color: var(--dna-ink-3); font-style: italic; font-size: 11px; }
.dna-deck__spacer { flex: 1; }
.dna-deck__chord {
  font-family: ui-monospace, monospace;
  background: var(--dna-paper-2);
  border: 1px solid var(--dna-line);
  border-radius: var(--dna-radius-chip);
  padding: 0 6px;
  color: var(--dna-ink-2);
}
"#;

#[cfg(test)]
mod tests {
    use dnacalc_skin_ir::identity::{NodeId, NodeKey};
    use dnacalc_skin_ir::workspace::{
        DefinedNameProjection, DefinedNameScopeProjection, DefinedNameTargetProjection,
        NodeContentKind, NodeValueProjection, NodeView, RevisionHistoryEntryProjection,
        SheetProjection,
    };

    use crate::composition::ShellComposition;
    use crate::profile::{ProfileTag, RuntimeContext};

    use super::*;

    fn registry() -> ShellKeyboardRegistry {
        ShellKeyboardRegistry::universal(
            ShellComposition::calc(ProfileTag::RichTree).catalog_composition(2),
            RuntimeContext::Browser,
        )
    }

    fn node(id: &str, key: &str, display: &str) -> NodeView {
        NodeView {
            key: NodeKey::new(key),
            id: NodeId::new(id),
            display_name: display.to_string(),
            parent: None,
            children: Vec::new(),
            depth: 0,
            content_kind: NodeContentKind::Empty,
            content_text: String::new(),
            computed_value: NodeValueProjection::Unevaluated,
            scenario_override: None,
            literalized_value_input: None,
            value_epoch: None,
            calc_state: None,
            effective_format: None,
            note: None,
            attributes: std::collections::BTreeMap::new(),
            binding_diagnostics: Vec::new(),
            is_meta: false,
            table: None,
        }
    }

    fn workspace() -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        let revenue = node("Accounts.2005.Revenue", "k1", "Revenue");
        let margin = node("Accounts.2005.Margin", "k2", "Margin");
        ws.node_order = vec![revenue.id.clone(), margin.id.clone()];
        ws.nodes.insert(revenue.id.clone(), revenue);
        ws.nodes.insert(margin.id.clone(), margin);
        ws.sheets = vec![SheetProjection {
            grid_node_id: NodeId::new("sheet:Summary"),
            display_name: "Summary".to_string(),
            position: 0,
        }];
        ws.defined_names.entries = vec![DefinedNameProjection {
            scope: DefinedNameScopeProjection::Workbook,
            name: "TaxRate".to_string(),
            target: DefinedNameTargetProjection::Dynamic {
                source_text: "0.2".to_string(),
            },
            is_dynamic: true,
        }];
        // A revision history that enables both undo (current r2 has parent r1)
        // and redo (r3's parent is the current r2), so the deck's undo/redo
        // commands are enabled against this populated host.
        ws.revision_history.current_revision_id = Some("r2".to_string());
        ws.revision_history.entries = vec![
            rev("r1", None, false),
            rev("r2", Some("r1"), true),
            rev("r3", Some("r2"), false),
        ];
        ws
    }

    fn rev(id: &str, parent: Option<&str>, is_current: bool) -> RevisionHistoryEntryProjection {
        RevisionHistoryEntryProjection {
            revision_id: id.to_string(),
            parent_revision_id: parent.map(str::to_string),
            structural_snapshot_id: "s".to_string(),
            node_input_snapshot_id: "n".to_string(),
            namespace_snapshot_id: "ns".to_string(),
            transaction_id: None,
            transaction_summary: None,
            is_current,
        }
    }

    fn inputs<'a>(
        ws: &'a WorkspaceState,
        sel: &'a SelectionState,
        registry: &'a ShellKeyboardRegistry,
        persona: Persona,
    ) -> DeckInputs<'a> {
        DeckInputs {
            workspace: ws,
            selection: sel,
            persona,
            visible_stages: vec![(1, "Model"), (2, "Atlas")],
            theme: Theme::CockpitLight,
            density: Density::Working,
            registry,
            host_can_save: false,
            host_can_open: false,
        }
    }

    #[test]
    fn static_command_id_set_is_the_pinned_agent_api() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let ids = static_command_ids(&inputs(&ws, &sel, &registry, Persona::Author));
        // Snapshot: ids are agent API — a rename is a breaking change and must
        // show up as a diff here.
        let expected = [
            "recalculate",
            "undo",
            "redo",
            "new_workspace",
            "shell.stage.1",
            "shell.stage.2",
            "shell.registry.toggle",
            "shell.inspector.toggle",
            "shell.atlas.open",
            "shell.timeline.open",
            "shell.theme.cockpit-light",
            "shell.theme.cockpit-dark",
            "shell.theme.high-contrast",
            "shell.density.working",
            "shell.density.reading",
            "shell.persona.author",
            "shell.persona.reviewer",
            "shell.persona.read_only",
            "shell.save",
            "shell.open",
        ];
        assert_eq!(ids, expected);
        assert!(
            ids.len() >= 15,
            "SHELL_SPEC 10.4 requires >= 15 commands, got {}",
            ids.len()
        );
    }

    #[test]
    fn at_least_fifteen_commands_are_enabled_against_a_populated_host() {
        let ws = workspace();
        // Select a node so selection-context commands enable.
        let sel = SelectionState::with_primary(Some(NodeId::new("Accounts.2005.Revenue")));
        let registry = registry();
        let commands = static_commands(&inputs(&ws, &sel, &registry, Persona::Author));
        let enabled = commands.iter().filter(|c| c.enabled).count();
        assert!(
            enabled >= 15,
            "expected >= 15 enabled commands, got {enabled}"
        );
    }

    #[test]
    fn effective_chords_come_from_the_live_registry() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let commands = static_commands(&inputs(&ws, &sel, &registry, Persona::Author));
        let recalc = commands.iter().find(|c| c.id == "recalculate").unwrap();
        assert_eq!(recalc.chord.as_deref(), Some("F9"));
        let stage1 = commands.iter().find(|c| c.id == "shell.stage.1").unwrap();
        assert_eq!(stage1.chord.as_deref(), Some("Ctrl+Alt+1"));
        let registry_toggle = commands
            .iter()
            .find(|c| c.id == "shell.registry.toggle")
            .unwrap();
        assert_eq!(registry_toggle.chord.as_deref(), Some("Ctrl+B"));
    }

    #[test]
    fn save_and_open_are_disabled_with_an_honest_reason_when_host_lacks_capability() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let commands = static_commands(&inputs(&ws, &sel, &registry, Persona::Author));
        let save = commands.iter().find(|c| c.id == "shell.save").unwrap();
        assert!(
            !save.enabled,
            "shell.save must be disabled until host support"
        );
        assert!(save.disabled_reason.is_some());
        assert_eq!(
            save.action,
            DeckAction::EmitShellIntent(SkinShellIntent::Save)
        );

        let open = commands.iter().find(|c| c.id == "shell.open").unwrap();
        assert!(
            !open.enabled,
            "shell.open must be disabled until host support"
        );
        assert!(open.disabled_reason.is_some());
        assert_eq!(
            open.action,
            DeckAction::EmitShellIntent(SkinShellIntent::Open {
                requested_path: None
            })
        );
    }

    /// The other half of the "two states" acceptance (bead dtc-lfz.3): when
    /// the host DOES advertise Save/Open capability, the deck entries flip
    /// to enabled with no disabled reason — the SAME real `SkinShellIntent`
    /// action as the disabled case (proving the action is wired
    /// unconditionally; only enablement tracks the flag). This is the arm
    /// `save_and_open_are_disabled_with_an_honest_reason_when_host_lacks_capability`
    /// does not cover — together they prove enablement genuinely TRACKS the
    /// advertised capability rather than being hardcoded false.
    #[test]
    fn save_and_open_enable_when_host_advertises_capability() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let mut deck_inputs = inputs(&ws, &sel, &registry, Persona::Author);
        deck_inputs.host_can_save = true;
        deck_inputs.host_can_open = true;
        let commands = static_commands(&deck_inputs);

        let save = commands.iter().find(|c| c.id == "shell.save").unwrap();
        assert!(
            save.enabled,
            "shell.save must enable once the host advertises can_save"
        );
        assert!(save.disabled_reason.is_none());
        assert_eq!(
            save.action,
            DeckAction::EmitShellIntent(SkinShellIntent::Save)
        );

        let open = commands.iter().find(|c| c.id == "shell.open").unwrap();
        assert!(
            open.enabled,
            "shell.open must enable once the host advertises can_open"
        );
        assert!(open.disabled_reason.is_none());
        assert_eq!(
            open.action,
            DeckAction::EmitShellIntent(SkinShellIntent::Open {
                requested_path: None
            })
        );
    }

    /// Mixed capability (can_save without can_open, and vice versa) proves
    /// the two entries are governed independently — neither is derived from
    /// the other, matching `PersistenceProjection`'s two independent fields.
    #[test]
    fn save_and_open_enablement_is_independent() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();

        let mut save_only = inputs(&ws, &sel, &registry, Persona::Author);
        save_only.host_can_save = true;
        save_only.host_can_open = false;
        let commands = static_commands(&save_only);
        assert!(
            commands
                .iter()
                .find(|c| c.id == "shell.save")
                .unwrap()
                .enabled
        );
        assert!(
            !commands
                .iter()
                .find(|c| c.id == "shell.open")
                .unwrap()
                .enabled
        );

        let mut open_only = inputs(&ws, &sel, &registry, Persona::Author);
        open_only.host_can_save = false;
        open_only.host_can_open = true;
        let commands = static_commands(&open_only);
        assert!(
            !commands
                .iter()
                .find(|c| c.id == "shell.save")
                .unwrap()
                .enabled
        );
        assert!(
            commands
                .iter()
                .find(|c| c.id == "shell.open")
                .unwrap()
                .enabled
        );
    }

    /// `run_action` genuinely dispatches through `ctx.shell_intent` for
    /// `EmitShellIntent` — proving the wiring is real, not merely a variant
    /// that looks wired. Constructs a full `OverlayContext` (the Leptos
    /// reactive types involved — `ReadSignal`/`RwSignal`/`Callback` — are
    /// plain thread-local arena handles in this Leptos version and need no
    /// mounted component/DOM to exercise).
    #[test]
    fn run_action_emit_shell_intent_calls_the_shell_intent_callback() {
        use std::sync::{Arc as StdArc, Mutex};

        use dnacalc_skin_ir::dispatcher::RecordingDispatcher;
        use dnacalc_skin_ir::state::SharedSkinState;
        use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
        use leptos::prelude::*;

        use crate::overlay::{OverlayContext, ShellControls};

        let received: StdArc<Mutex<Vec<SkinShellIntent>>> = StdArc::new(Mutex::new(Vec::new()));
        let received_for_cb = received.clone();
        let shell_intent_cb: Callback<SkinShellIntent> =
            Callback::new(move |intent: SkinShellIntent| {
                received_for_cb.lock().unwrap().push(intent);
            });

        let workspace = RwSignal::new(WorkspaceState::default());
        let selection = RwSignal::new(SelectionState::default());
        let shared = SharedSkinStateHandle::new(SharedSkinState::default());
        let noop = Callback::new(move |_: ()| {});
        let noop_u8 = Callback::new(move |_: u8| {});
        let noop_theme = Callback::new(move |_: Theme| {});
        let noop_density = Callback::new(move |_: Density| {});

        let ctx = OverlayContext {
            shared,
            dispatch: StdArc::new(RecordingDispatcher::default()),
            goto_mode: false,
            workspace: workspace.read_only(),
            selection: selection.read_only(),
            keyboard: registry(),
            theme: Theme::CockpitLight,
            density: Density::Working,
            visible_stages: Vec::new(),
            host_can_save: true,
            host_can_open: true,
            shell_intent: Some(shell_intent_cb),
            controls: ShellControls {
                switch_stage: noop_u8,
                toggle_registry: noop,
                toggle_inspector: noop,
                open_atlas: noop,
                open_timeline: noop,
                set_theme: noop_theme,
                set_density: noop_density,
                close: noop,
            },
        };

        run_action(&ctx, &DeckAction::EmitShellIntent(SkinShellIntent::Save));
        assert_eq!(*received.lock().unwrap(), vec![SkinShellIntent::Save]);

        // The disabled/unwired case: `shell_intent` is `None` — `run_action`
        // must not panic (mirrors the `Inert` no-op discipline above).
        let mut ctx_unwired = ctx;
        ctx_unwired.shell_intent = None;
        run_action(
            &ctx_unwired,
            &DeckAction::EmitShellIntent(SkinShellIntent::Open {
                requested_path: None,
            }),
        );
        assert_eq!(
            received.lock().unwrap().len(),
            1,
            "an unwired channel must not call back or panic"
        );
    }

    #[test]
    fn deck_enablement_equals_dispatch_acceptance_across_all_personas() {
        let ws = workspace();
        let sel = SelectionState::with_primary(Some(NodeId::new("Accounts.2005.Revenue")));
        let registry = registry();
        for persona in [Persona::Author, Persona::Reviewer, Persona::ReadOnly] {
            let commands = static_commands(&inputs(&ws, &sel, &registry, persona));
            // The authoritative expectation: the catalog governed by this
            // persona — which mirrors the dispatcher's persona gate
            // (`Persona::allows_intent_kind`, itself pinned to `allows` over
            // real intents). The deck must not re-derive enablement; it must
            // equal the governed catalog for every runnable catalog command.
            let governed = ws.command_catalog(&sel).governed_by(persona);
            for kind in RUNNABLE_CATALOG_KINDS {
                let deck_command = commands
                    .iter()
                    .find(|command| command.id == kind.stable_id())
                    .unwrap();
                let catalog_enabled = governed.get(kind).unwrap().enabled;
                assert_eq!(
                    deck_command.enabled,
                    catalog_enabled,
                    "deck enablement must equal the governed catalog (= dispatch \
                     acceptance) for {} / {persona}",
                    kind.stable_id()
                );
            }
        }
    }

    #[test]
    fn reviewer_loses_recalc_is_kept_readonly_loses_everything() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();

        // Reviewer keeps Recalculate (a governed-allowed catalog command).
        let reviewer = static_commands(&inputs(&ws, &sel, &registry, Persona::Reviewer));
        assert!(
            reviewer
                .iter()
                .find(|c| c.id == "recalculate")
                .unwrap()
                .enabled
        );

        // ReadOnly loses every catalog command (governed_by disables them).
        let read_only = static_commands(&inputs(&ws, &sel, &registry, Persona::ReadOnly));
        for id in ["recalculate", "undo", "redo", "new_workspace"] {
            let command = read_only.iter().find(|c| c.id == id).unwrap();
            assert!(!command.enabled, "read-only must not run {id}");
            assert_eq!(
                command.disabled_reason.as_deref(),
                Some("the read_only persona cannot run this command")
            );
        }
    }

    #[test]
    fn parse_a1_grammar_table() {
        // Bare cell.
        assert_eq!(
            parse_a1("A1"),
            Some(A1Address {
                sheet: None,
                col: 1,
                row: 1
            })
        );
        // Sheet-qualified.
        assert_eq!(
            parse_a1("Sheet2!C4"),
            Some(A1Address {
                sheet: Some("Sheet2".to_string()),
                col: 3,
                row: 4
            })
        );
        // Multi-letter column (AA = 27).
        assert_eq!(parse_a1("AA10").map(|a| a.col), Some(27));
        // Case-insensitive column letters.
        assert_eq!(parse_a1("c4").map(|a| a.col), Some(3));
        // A plain name is NOT an A1 ref — falls through to name/path goto.
        assert_eq!(parse_a1("Revenue"), None);
        // A dotted node path is NOT an A1 ref.
        assert_eq!(parse_a1("Accounts.2005.Margin"), None);
        // Degenerate forms rejected.
        assert_eq!(parse_a1("A0"), None); // row 0
        assert_eq!(parse_a1("1A"), None); // digits before letters
        assert_eq!(parse_a1("A"), None); // no row
        assert_eq!(parse_a1("!C4"), None); // empty sheet
        assert_eq!(parse_a1("Sheet2!"), None); // empty cell
    }

    #[test]
    fn a1_canonicalizes_round_trip() {
        assert_eq!(parse_a1("aa10").unwrap().canonical(), "AA10");
        assert_eq!(parse_a1("sheet 1!b2").unwrap().canonical(), "sheet 1!B2");
        assert_eq!(parse_a1("A1").unwrap().canonical(), "A1");
    }

    #[test]
    fn goto_matches_a1_sheets_names_and_node_paths() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let inputs = inputs(&ws, &sel, &registry, Persona::Author);

        // A1 ref → a select-cell navigation entry with the canonical target.
        let a1 = goto_commands(&inputs, "C4");
        assert_eq!(a1.len(), 1);
        assert_eq!(a1[0].id, "goto:a1:C4");
        assert_eq!(
            a1[0].action,
            DeckAction::Dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new("C4"))))
        );

        // Sheet name match.
        let sheet = goto_commands(&inputs, "summ");
        assert!(sheet.iter().any(|c| c.id == "goto:sheet:sheet:Summary"));

        // Defined name match (Name-Box mirror: SelectNode by name string).
        let name = goto_commands(&inputs, "tax");
        let tax = name.iter().find(|c| c.id == "goto:name:TaxRate").unwrap();
        assert_eq!(
            tax.action,
            DeckAction::Dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new("TaxRate"))))
        );

        // Node path / display-name match — "Revenue" hits the node's display.
        let node = goto_commands(&inputs, "revenue");
        assert!(
            node.iter()
                .any(|c| c.id == "goto:node:Accounts.2005.Revenue")
        );

        // Empty query → no goto entries.
        assert!(goto_commands(&inputs, "   ").is_empty());
    }

    #[test]
    fn fuzzy_filter_is_deterministic_and_prefers_prefix_matches() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let commands = static_commands(&inputs(&ws, &sel, &registry, Persona::Author));

        let once = filter_commands("re", commands.clone());
        let twice = filter_commands("re", commands.clone());
        assert_eq!(
            once.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
            twice.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
            "identical inputs must yield identical ranking"
        );

        // "re" is a prefix of "Recalculate" and "Redo" — both outrank the
        // scattered match in "Toggle registry rail".
        let titles: Vec<&str> = once.iter().map(|c| c.title.as_str()).collect();
        let recalc = titles.iter().position(|t| *t == "Recalculate").unwrap();
        let toggle = titles.iter().position(|t| t.contains("registry")).unwrap();
        assert!(
            recalc < toggle,
            "prefix match must outrank a scattered subsequence: {titles:?}"
        );

        // A non-subsequence query drops everything.
        assert!(filter_commands("zzzq", commands.clone()).is_empty());

        // Empty query keeps every command in original order.
        let all = filter_commands("", commands.clone());
        assert_eq!(all.len(), commands.len());
        assert_eq!(all[0].id, commands[0].id);
    }

    #[test]
    fn fuzzy_score_orders_prefix_above_contiguous_above_scattered() {
        let prefix = fuzzy_score("com", "Command deck").unwrap();
        let scattered = fuzzy_score("com", "Toggle inspector comment").unwrap();
        assert!(
            prefix > scattered,
            "prefix {prefix} should beat scattered {scattered}"
        );
        assert_eq!(fuzzy_score("xyz", "Command deck"), None);
    }

    #[test]
    fn deck_listing_puts_goto_entries_before_commands() {
        let ws = workspace();
        let sel = SelectionState::default();
        let registry = registry();
        let inputs = inputs(&ws, &sel, &registry, Persona::Author);
        // "re" matches the Revenue node (goto) and command titles (Recalculate,
        // Redo, …) — goto entries must come first.
        let listing = deck_listing(&inputs, "re");
        assert_eq!(listing[0].group, DeckGroup::Goto);
        assert!(listing.iter().any(|c| c.group == DeckGroup::Command));
    }
}
