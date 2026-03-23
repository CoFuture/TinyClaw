//! Terminal UI module
//! 
//! Provides an interactive terminal interface for TinyClaw.

mod app;
mod components;
mod gateway_client;
mod markdown;
mod persistence;
mod state;

pub use app::run_tui;
#[allow(unused_imports)]
pub use {app::TuiApp, gateway_client::{TuiGatewayClient, TuiGatewayEvent, SessionInfo, TuiGatewayStatus}, persistence::TuiPersistence, state::AppState};
