//! Terminal UI module
//! 
//! Provides an interactive terminal interface for TinyClaw.

mod app;
mod components;
mod state;

pub use app::run_tui;
#[allow(unused_imports)]
pub use {app::TuiApp, state::AppState};
