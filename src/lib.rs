//! TinyClaw - A minimal implementation of OpenClaw in Rust

pub mod agent;
pub mod common;
pub mod config;
pub mod gateway;
pub mod http;
pub mod plugins;
pub mod tui;

pub use common::error::Error;
