//! # MDT — Minecraft Download Toolkit
//!
//! [![Crates.io](https://img.shields.io/crates/v/mdt-minecraft.svg)](https://crates.io/crates/mdt-minecraft)
//! [![Documentation](https://docs.rs/mdt-minecraft/badge.svg)](https://docs.rs/mdt-minecraft)
//! [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
//!
//! An ergonomic, low-level library that downloads and installs Minecraft versions and mod
//! loaders into a clean, isolated `.minecraft` layout. Build a launcher on top of it in three
//! lines:
//!
//! ```no_run
//! use mdt::{MdtClient, Mirror, ModLoader};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = MdtClient::builder()
//!         .mirror(Mirror::BMCLAPI)
//!         .root_dir("./.minecraft")
//!         .build()?;
//!
//!     client.install_vanilla("1.20.1").await?;
//!     client.install_modloader("1.20.1", ModLoader::Fabric, "0.15.11").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Isolation guarantee
//!
//! Every loader install lands in `versions/<mc_version>+<Loader><loader_version>` (for example
//! `versions/1.16.5+Fabric0.20.0`). The vanilla directory `versions/<mc_version>` is never
//! touched by a loader install, so a broken loader can never corrupt the underlying game.
//!
//! # Errors
//!
//! Every fallible operation returns [`Result<T, MdtError>`][std::result::Result], so downstream
//! launchers can pattern-match on the exact failure (network, checksum mismatch, missing Java,
//! ...) instead of handling a boxed catch-all. [`MdtError`] is `#[non_exhaustive]`, so match
//! arms should include a wildcard `_` arm to stay forward-compatible.
//!
//! # Mirrors and custom providers
//!
//! The download source is chosen with [`Mirror`]: [`Mirror::BMCLAPI`] (the default — a community
//! mirror that proxies Mojang, Fabric, Forge, NeoForge, and LiteLoader), [`Mirror::Mojang`] (the
//! official upstream), or [`Mirror::Custom`] — a provider you declare yourself. Every provider's
//! endpoint URLs and URL-rewrite rules live in `providers.toml`, and the `mdt_macros::providers!`
//! proc macro compiles them into `&'static` lookup tables at build time, so nothing is read or
//! parsed at runtime.
//!
//! To register a custom mirror, add a `[name]` section to `providers.toml`, rebuild, and select it
//! at runtime:
//!
//! ```no_run
//! use mdt::{MdtClient, Mirror};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = MdtClient::builder()
//!         .mirror(Mirror::Custom("my_mirror".into()))
//!         .root_dir("./.minecraft")
//!         .build()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! [`Mirror::provider_name`] returns the active provider's `[name]` key, and failed requests fall
//! back automatically via [`Mirror::other`] (a custom mirror falls back to `BMCLAPI`). The full
//! layout is specified in [the TOML spec](https://github.com/Lavaver/MDT/blob/main/docs/mirror_toml.md)
//! and [the endpoint URI spec](https://github.com/Lavaver/MDT/blob/main/docs/mirror_uri.md).
//!
//! # Macros
//!
//! [`version_dir!`], [`download_task!`], and [`mdt_install!`] provide one-line ergonomic
//! construction for isolated paths, download tasks, and loader installs.

mod client;
mod error;
mod http;
mod macros;

pub mod downloader;
pub mod installer;
pub mod manifest;
pub mod mc_version;
pub mod mirror;
pub mod modloader;
pub mod urls;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "tui")]
pub mod tui;

pub use client::{MdtClient, MdtClientBuilder};
pub use downloader::{AssetIndex, AssetObject, DownloadTask, VersionJson};
pub use error::MdtError;
pub use installer::{InstallMethod, InstallRequest, InstalledVersion};
pub use manifest::{
    FabricVersionsList, ForgeVersionsList, LatestMinecraftVersion, LiteLoaderVersionEntry,
    LoaderVersionEntry, ManifestData, MavenMetadata, MinecraftVersionsList, QuiltVersionsList,
    VersionEntry, VersionType, fetch_all_manifests, loader_versions_from_data, neoforge_mc_prefix,
};
pub use mc_version::McVersion;
pub use mirror::Mirror;
pub use modloader::ModLoader;
pub use urls::ManifestUrls;
