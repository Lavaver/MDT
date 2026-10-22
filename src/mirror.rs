//! Download mirror sources.

/// The upstream mirror from which game files and metadata are downloaded.
///
/// [`Mirror::BMCLAPI`] is the default and is generally faster for users in mainland
/// China; [`Mirror::Mojang`] is the official upstream. When a request against the
/// active mirror fails, MDT automatically retries against [`Mirror::other`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mirror {
    /// Official Mojang servers.
    Mojang,
    /// The BMCLAPI community mirror (proxies Mojang, Fabric, Forge, NeoForge, and LiteLoader).
    #[default]
    BMCLAPI,
}

impl Mirror {
    /// The opposite mirror, used as an automatic fallback when the primary source fails.
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Mirror::Mojang => Mirror::BMCLAPI,
            Mirror::BMCLAPI => Mirror::Mojang,
        }
    }
}
