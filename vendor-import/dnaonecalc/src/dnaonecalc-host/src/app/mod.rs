use crate::state::OneCalcHostState;

pub mod case_lifecycle;
pub mod host_mount;
pub mod intents;
pub mod preview_state;
pub mod reducer;

#[derive(Debug, Default)]
pub struct OneCalcHostApp {
    state: OneCalcHostState,
}

impl OneCalcHostApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &OneCalcHostState {
        &self.state
    }

    pub fn launch_message(&self) -> &'static str {
        "DNA OneCalc shared app core scaffold initialized"
    }
}
