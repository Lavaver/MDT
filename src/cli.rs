//! Command-line interface surface, compiled only under the `cli` feature.
//!
//! The core library is fully terminal-agnostic. Argument parsing and the terminal-driven
//! launcher live here so that consumers who only need the library never pay the compile cost of
//! [`clap`], [`crossterm`], or [`ratatui`].

/// Re-export of [`clap::Parser`] so downstream CLI launchers can derive their argument structs.
pub use clap::Parser;
