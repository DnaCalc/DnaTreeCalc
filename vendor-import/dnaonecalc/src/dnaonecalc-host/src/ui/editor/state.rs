#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionAggressiveness {
    Manual,
    OnIdentifier,
    Always,
}

impl CompletionAggressiveness {
    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::OnIdentifier => "On identifier",
            Self::Always => "Always",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::OnIdentifier => "on-identifier",
            Self::Always => "always",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpPlacement {
    Inline,
    Sidecar,
}

impl HelpPlacement {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inline => "Inline",
            Self::Sidecar => "Sidecar",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Sidecar => "sidecar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorSettings {
    pub auto_close_brackets: bool,
    pub highlight_bracket_pairs: bool,
    pub completion_aggressiveness: CompletionAggressiveness,
    pub help_placement: HelpPlacement,
    pub reuse_timing_badge_visible: bool,
    pub reduce_motion: bool,
    pub auto_proof_quiet_interval_ms: Option<u32>,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            auto_close_brackets: true,
            highlight_bracket_pairs: true,
            completion_aggressiveness: CompletionAggressiveness::OnIdentifier,
            help_placement: HelpPlacement::Sidecar,
            reuse_timing_badge_visible: false,
            reduce_motion: false,
            auto_proof_quiet_interval_ms: Some(600),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorSettingUpdate {
    ToggleAutoCloseBrackets,
    ToggleHighlightBracketPairs,
    SetCompletionAggressiveness(CompletionAggressiveness),
    SetHelpPlacement(HelpPlacement),
    ToggleReuseTimingBadge,
    ToggleReduceMotion,
    ToggleAutoProofQuietInterval,
    SetAutoProofQuietIntervalMs(u32),
}

impl EditorSettings {
    pub fn apply(&mut self, update: EditorSettingUpdate) {
        match update {
            EditorSettingUpdate::ToggleAutoCloseBrackets => {
                self.auto_close_brackets = !self.auto_close_brackets;
            }
            EditorSettingUpdate::ToggleHighlightBracketPairs => {
                self.highlight_bracket_pairs = !self.highlight_bracket_pairs;
            }
            EditorSettingUpdate::SetCompletionAggressiveness(value) => {
                self.completion_aggressiveness = value;
            }
            EditorSettingUpdate::SetHelpPlacement(value) => {
                self.help_placement = value;
            }
            EditorSettingUpdate::ToggleReuseTimingBadge => {
                self.reuse_timing_badge_visible = !self.reuse_timing_badge_visible;
            }
            EditorSettingUpdate::ToggleReduceMotion => {
                self.reduce_motion = !self.reduce_motion;
            }
            EditorSettingUpdate::ToggleAutoProofQuietInterval => {
                self.auto_proof_quiet_interval_ms = if self.auto_proof_quiet_interval_ms.is_some() {
                    None
                } else {
                    Some(600)
                };
            }
            EditorSettingUpdate::SetAutoProofQuietIntervalMs(ms) => {
                self.auto_proof_quiet_interval_ms = Some(ms);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorEntryMode {
    Formula,
    Value,
    Text,
    Empty,
}

impl EditorEntryMode {
    pub fn classify(text: &str) -> Self {
        let trimmed = text.trim_start_matches([' ', '\t']);
        if trimmed.is_empty() {
            Self::Empty
        } else if trimmed.starts_with('=') {
            Self::Formula
        } else if trimmed.starts_with('\'') {
            Self::Text
        } else {
            Self::Value
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Formula => "Formula",
            Self::Value => "Value",
            Self::Text => "Text",
            Self::Empty => "Empty",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::Value => "value",
            Self::Text => "text",
            Self::Empty => "empty",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorLiveState {
    Idle,
    EditingLive,
    ProofedScratch,
    Committed,
}

impl EditorLiveState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::EditingLive => "Editing",
            Self::ProofedScratch => "Proofed",
            Self::Committed => "Committed",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::EditingLive => "editing-live",
            Self::ProofedScratch => "proofed-scratch",
            Self::Committed => "committed",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::Idle => "·",
            Self::EditingLive => "✏",
            Self::ProofedScratch => "⟳",
            Self::Committed => "✓",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCaret {
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl EditorSelection {
    pub fn collapsed(offset: usize) -> Self {
        Self {
            anchor: offset,
            focus: offset,
        }
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.focus)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.focus)
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorScrollWindow {
    pub first_visible_line: usize,
    pub visible_line_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSurfaceState {
    pub caret: EditorCaret,
    pub selection: EditorSelection,
    pub scroll_window: EditorScrollWindow,
    pub completion_anchor_offset: Option<usize>,
    pub completion_selected_index: Option<usize>,
    pub signature_help_anchor_offset: Option<usize>,
}

impl EditorSurfaceState {
    pub fn for_text(text: &str) -> Self {
        let offset = text.chars().count();
        Self::for_text_with_selection(text, offset, offset)
    }

    pub fn for_text_with_selection(text: &str, anchor: usize, focus: usize) -> Self {
        let text_len = text.chars().count();
        let anchor = anchor.min(text_len);
        let focus = focus.min(text_len);
        let line_count = text.lines().count().max(1);
        Self {
            caret: EditorCaret { offset: focus },
            selection: EditorSelection { anchor, focus },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: line_count.min(12),
            },
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_surface_state_defaults_to_end_of_text() {
        let state = EditorSurfaceState::for_text("=SUM(1,2)");
        assert_eq!(state.caret.offset, 9);
        assert!(state.selection.is_collapsed());
        assert_eq!(state.scroll_window.first_visible_line, 0);
        assert!(state.completion_anchor_offset.is_none());
        assert!(state.completion_selected_index.is_none());
        assert!(state.signature_help_anchor_offset.is_none());
    }

    #[test]
    fn editor_surface_state_can_preserve_explicit_selection() {
        let state = EditorSurfaceState::for_text_with_selection("=SUM(1,2)", 2, 5);
        assert_eq!(state.selection.anchor, 2);
        assert_eq!(state.selection.focus, 5);
        assert_eq!(state.caret.offset, 5);
        assert!(state.completion_anchor_offset.is_none());
        assert!(state.completion_selected_index.is_none());
        assert!(state.signature_help_anchor_offset.is_none());
    }
}
