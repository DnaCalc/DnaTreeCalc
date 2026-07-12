//! End-to-end scenario tests for the OneCalc view-model layer.
//!
//! Each scenario walks a user action from `OneCalcHostState::default()`
//! through the reducer / live-edit entry points (using the real
//! `NativeOxfmlHostSession` where possible), builds the home-shell projection,
//! and asserts on user-visible result fields only. Cross-layer composition
//! is the point; tests are named for the user action.

#[path = "scenarios/typing.rs"]
mod typing;

#[path = "scenarios/completion.rs"]
mod completion;

#[path = "scenarios/runtime_metadata.rs"]
mod runtime_metadata;
