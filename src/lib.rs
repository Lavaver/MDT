//! # MDT — Minecraft Download Toolkit
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
