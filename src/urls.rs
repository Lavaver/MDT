//! Mirror-aware URL construction and rewriting.

use crate::mirror::Mirror;

/// The set of endpoints and download base URLs for the active mirror.
#[derive(Debug, Clone)]
pub struct ManifestUrls {
    // ===== manifest (metadata) endpoints =====
    pub minecraft: String,
    pub forge: String,
    pub fabric: String,
    pub quilt: String,
    pub neoforge: String,
    pub liteloader: String,

    // ===== loader download base URLs =====
    pub forge_maven: String,
    pub fabric_maven: String,
    pub quilt_maven: String,
    pub neoforge_maven: String,
    pub liteloader_base: String,

    // ===== metadata bases used to join profile sub-endpoints =====
    pub fabric_meta: String,
    pub quilt_meta: String,
}

impl ManifestUrls {
    /// Build the endpoint table for the given mirror.
    #[must_use]
    pub fn new(mirror: Mirror) -> Self {
        match mirror {
            Mirror::Mojang => Self {
                minecraft: "https://piston-meta.mojang.com/mc/game/version_manifest.json".to_string(),
                forge: "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json".to_string(),
                fabric: "https://meta.fabricmc.net/v2/versions".to_string(),
                quilt: "https://meta.quiltmc.org/v3/versions".to_string(),
                neoforge: "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml".to_string(),
                liteloader: "https://repo.mumfrey.com/content/repositories/snapshots/com/mumfrey/liteloader/maven-metadata.xml".to_string(),

                forge_maven: "https://maven.minecraftforge.net".to_string(),
                fabric_maven: "https://maven.fabricmc.net".to_string(),
                quilt_maven: "https://maven.quiltmc.org/repository/release".to_string(),
                neoforge_maven: "https://maven.neoforged.net/releases".to_string(),
                liteloader_base: "https://dl.liteloader.com/versions".to_string(),

                fabric_meta: "https://meta.fabricmc.net".to_string(),
                quilt_meta: "https://meta.quiltmc.org".to_string(),
            },
            Mirror::BMCLAPI => Self {
                minecraft: "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json".to_string(),
                forge: "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/maven-metadata.json".to_string(),
                fabric: "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions".to_string(),
                quilt: "https://bmclapi2.bangbang93.com/quilt-meta/v3/versions".to_string(),
                neoforge: "https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/maven-metadata.xml".to_string(),
                liteloader: "https://bmclapi2.bangbang93.com/liteloader/list".to_string(),

                forge_maven: "https://bmclapi2.bangbang93.com/maven".to_string(),
                fabric_maven: "https://bmclapi2.bangbang93.com/maven".to_string(),
                quilt_maven: "https://bmclapi2.bangbang93.com/maven".to_string(),
                neoforge_maven: "https://bmclapi2.bangbang93.com/maven".to_string(),
                liteloader_base: "https://bmclapi2.bangbang93.com/liteloader/download".to_string(),

                fabric_meta: "https://bmclapi2.bangbang93.com/fabric-meta".to_string(),
                quilt_meta: "https://bmclapi2.bangbang93.com/quilt-meta".to_string(),
            },
        }
    }

    /// The Fabric official-installer `profile` endpoint (returns the full version JSON config).
    ///
    /// Path format is `/v2/versions/loader/{game}/{loader}/profile/json`
    /// (game version first, then loader version).
    #[must_use]
    pub fn fabric_profile(&self, loader: &str, game: &str) -> String {
        format!("{}/v2/versions/loader/{}/{}/profile/json", self.fabric_meta, game, loader)
    }

    /// The Quilt official-installer `profile` endpoint.
    ///
    /// Path format is `/v3/versions/loader/{game}/{loader}/profile/json`.
    #[must_use]
    pub fn quilt_profile(&self, loader: &str, game: &str) -> String {
        format!("{}/v3/versions/loader/{}/{}/profile/json", self.quilt_meta, game, loader)
    }

    /// Compute the Quilt profile endpoint with mirror fallback.
    ///
    /// BMCLAPI does not proxy Quilt's `/v3/versions/loader/{game}/{loader}/profile/json`
    /// endpoint (it returns 404), so under [`Mirror::BMCLAPI`] this falls back to the official
    /// `meta.quiltmc.org`.
    #[must_use]
    pub fn quilt_profile_url(&self, loader: &str, game: &str, mirror: Mirror) -> String {
        match mirror {
            Mirror::BMCLAPI => ManifestUrls::new(Mirror::Mojang).quilt_profile(loader, game),
            Mirror::Mojang => self.quilt_profile(loader, game),
        }
    }

    /// Rewrite a Mojang download URL through the active mirror.
    #[must_use]
    pub fn apply_mirror(&self, original_url: &str, mirror: Mirror) -> String {
        match mirror {
            Mirror::Mojang => original_url.to_string(),
            Mirror::BMCLAPI => {
                original_url
                    .replace("https://libraries.minecraft.net", "https://bmclapi2.bangbang93.com/maven")
                    .replace("https://piston-data.mojang.com", "https://bmclapi2.bangbang93.com")
                    .replace("https://piston-meta.mojang.com", "https://bmclapi2.bangbang93.com")
                    .replace("https://launchermeta.mojang.com", "https://bmclapi2.bangbang93.com")
                    .replace("https://resources.download.minecraft.net", "https://bmclapi2.bangbang93.com/assets")
            }
        }
    }

    /// Rewrite a Maven repository base URL through the active mirror.
    ///
    /// BMCLAPI's `/maven` is a unified Maven proxy. Note `files.minecraftforge.net` exists in
    /// both a `/maven`-suffixed (old library base) and non-suffixed (manifest) form, so the
    /// suffixed form must be matched first to avoid path duplication.
    #[must_use]
    pub fn apply_maven_mirror(&self, url: &str, mirror: Mirror) -> String {
        const MIRROR_MAVEN: &str = "https://bmclapi2.bangbang93.com/maven";
        match mirror {
            Mirror::Mojang => url.to_string(),
            Mirror::BMCLAPI => {
                // BMCLAPI does not proxy the Quilt Maven (`org.quiltmc`); keep official URLs so
                // Quilt installs reach the official server instead of 404ing.
                if url.contains("maven.quiltmc.org") {
                    return url.to_string();
                }
                url.replace("http://files.minecraftforge.net/maven", MIRROR_MAVEN)
                    .replace("https://files.minecraftforge.net/maven", MIRROR_MAVEN)
                    .replace("https://files.minecraftforge.net", MIRROR_MAVEN)
                    .replace("https://maven.fabricmc.net", MIRROR_MAVEN)
                    .replace("https://maven.minecraftforge.net", MIRROR_MAVEN)
                    .replace("https://maven.neoforged.net", MIRROR_MAVEN)
                    .replace("https://libraries.minecraft.net", MIRROR_MAVEN)
            }
        }
    }
}
