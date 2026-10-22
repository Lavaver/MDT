//! Terminal UI surface, compiled only under the `tui` feature.
//!
//! Holds the interactive UI (progress rendering, selection wizards). Gated behind `tui` so that
//! `crossterm` and `ratatui` are never compiled unless explicitly requested.

pub use crossterm;
pub use ratatui;
