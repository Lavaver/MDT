//! Command-line interface, only compiled under the `cli` feature.
//!
//! The core library doesn't touch the terminal. Argument parsing lives here so library-only
//! users never pay for [`clap`], [`crossterm`], or [`ratatui`].

/// Re-export of [`clap::Parser`] so downstream CLI launchers can derive their argument structs.
pub use clap::Parser;
