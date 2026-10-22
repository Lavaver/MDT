//! Supported mod loaders.

use crate::mc_version::McVersion;

/// A supported Minecraft mod loader.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModLoader {
    /// Classic mod loader, covering roughly 1.5.2 through 1.21+.
    Forge,
    /// Lightweight, high-performance loader for 1.14+.
    Fabric,
    /// Community fork of Fabric focused on extensibility (1.14+).
    Quilt,
    /// Modern fork of Forge for 1.21+.
    Neoforge,
    /// Lightweight loader discontinued after 1.12.2.
    Liteloader,
}

impl ModLoader {
    /// The canonical display name, used in isolated version IDs and directory names
    /// (e.g. `1.20.1+Fabric0.15.11`).
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            ModLoader::Forge => "Forge",
            ModLoader::Fabric => "Fabric",
            ModLoader::Quilt => "Quilt",
            ModLoader::Neoforge => "NeoForge",
            ModLoader::Liteloader => "LiteLoader",
        }
    }

    /// Whether this loader supports the given Minecraft version.
    ///
    /// * **LiteLoader** — `1.12.2` and earlier.
    /// * **Fabric / Quilt** — `1.14` and later.
    /// * **NeoForge** — `1.21` and later.
    /// * **Forge** — all versions.
    ///
    /// Unparseable versions (e.g. weekly snapshots like `24w05a`) are treated as modern
    /// versions and only exclude LiteLoader.
    #[must_use]
    pub fn compatible_with(self, mc_version: &str) -> bool {
        let Some(v) = McVersion::parse(mc_version) else {
            return self != ModLoader::Liteloader;
        };
        match self {
            ModLoader::Forge => true,
            ModLoader::Liteloader => v <= McVersion::new(1, 12, 2),
            ModLoader::Fabric | ModLoader::Quilt => v >= McVersion::new(1, 14, 0),
            ModLoader::Neoforge => v >= McVersion::new(1, 21, 0),
        }
    }
}
