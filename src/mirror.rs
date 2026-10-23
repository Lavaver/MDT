//! Download mirror sources.

/// The upstream mirror from which game files and metadata are downloaded.
///
/// [`Mirror::BMCLAPI`] is the default and is generally faster for users in mainland
/// China; [`Mirror::Mojang`] is the official upstream. When a request against the
/// active mirror fails, MDT automatically retries against [`Mirror::other`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Mirror {
    /// Official Mojang servers.
    Mojang,
    /// The BMCLAPI community mirror (proxies Mojang, Fabric, Forge, NeoForge, and LiteLoader).
    #[default]
    BMCLAPI,
    /// A custom mirror provider, keyed by its `[provider_name]` section in `providers.toml`.
    ///
    /// The name must match a section compiled in at build time — see the
    /// [TOML specification](/docs/mirror_toml.md).
    Custom(String),
}

impl Mirror {
    /// The opposite mirror, used as an automatic fallback when the primary source fails.
    #[must_use]
    pub fn other(&self) -> Self {
        match self {
            Mirror::Mojang => Mirror::BMCLAPI,
            Mirror::BMCLAPI => Mirror::Mojang,
            // A custom mirror has no single "opposite"; fall back to the default mirror.
            Mirror::Custom(_) => Mirror::BMCLAPI,
        }
    }

    /// The name of this mirror's provider, as declared in `providers.toml`.
    ///
    /// The name is the `[provider_name]` key used to look up the provider's endpoint URLs and
    /// rewrite rules (see [`crate::urls::ManifestUrls`]). Built-in providers return `"mojang"`
    /// and `"bmclapi"`; [`Mirror::Custom`] returns its own name.
    ///
    /// - For the URI specification for providers, see [this document](/docs/mirror_uri.md).
    /// - For the TOML specification for providers, see [this document](/docs/mirror_toml.md).
    #[must_use]
    pub fn provider_name(&self) -> &str {
        match self {
            Mirror::Mojang => "mojang",
            Mirror::BMCLAPI => "bmclapi",
            Mirror::Custom(name) => name.as_str(),
        }
    }
}
