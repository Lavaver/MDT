//! The [`MdtClient`] and its builder.

use std::path::{Path, PathBuf};

use crate::downloader::{self, AssetIndex, VersionJson};
use crate::error::MdtError;
use crate::http;
use crate::installer::{self, InstallRequest, InstalledVersion};
use crate::manifest::{self, VersionEntry};
use crate::mirror::Mirror;
use crate::modloader::ModLoader;
use crate::urls::ManifestUrls;

/// Default download concurrency.
///
/// Three is enough for most connections; go higher and some mirrors will rate-limit or 403 you.
const DEFAULT_CONCURRENCY: usize = 3;

/// The `MdtClient`. One instance holds everything a download needs: a pooled HTTP client, the
/// active [`Mirror`], and the `.minecraft` root directory. Build one with [`MdtClient::builder`]
/// and reuse it for as many installs as you want.
///
/// ```no_run
/// use mdt::{MdtClient, Mirror, ModLoader};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = MdtClient::builder()
///         .mirror(Mirror::BMCLAPI)
///         .root_dir("./.minecraft")
///         .build()?;
///
///     client.install_vanilla("1.20.1").await?;
///     client.install_modloader("1.20.1", ModLoader::Fabric, "0.15.11").await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MdtClient {
    http: reqwest::Client,
    mirror: Mirror,
    root: PathBuf,
    concurrency: usize,
}

impl MdtClient {
    /// Create a new [`MdtClientBuilder`].
    #[must_use]
    pub fn builder() -> MdtClientBuilder {
        MdtClientBuilder::new()
    }

    /// The shared HTTP client (connection-pooled, with a browser User-Agent).
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The active mirror source.
    pub fn mirror(&self) -> &Mirror {
        &self.mirror
    }

    /// The target `.minecraft` root directory.
    pub fn root_dir(&self) -> &Path {
        &self.root
    }

    /// The configured download concurrency.
    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Change the download concurrency at runtime (clamped to at least 1).
    ///
    /// Unlike [`MdtClientBuilder::concurrency`], this works after the client is built — handy for
    /// a user preference or a `--jobs`-style CLI flag.
    pub fn set_concurrency(&mut self, concurrency: usize) {
        self.concurrency = concurrency.max(1);
    }

    /// Fetch the list of available Minecraft versions.
    pub async fn versions(&self) -> Result<Vec<VersionEntry>, MdtError> {
        let manifest = manifest::fetch_minecraft_version_manifest(&self.http, &self.mirror).await?;
        Ok(manifest.versions)
    }

    /// Download and install a vanilla Minecraft version.
    ///
    /// Returns the parsed [`VersionJson`]. Downloads the client JAR, libraries (rules and natives
    /// extraction included), and asset index, and writes the version JSON to
    /// `versions/<id>/<id>.json` so launchers and loader installers can read it back.
    ///
    /// # Errors
    ///
    /// * [`MdtError::VersionNotFound`] if `version` isn't in the manifest.
    /// * [`MdtError::NetworkError`] / [`MdtError::ChecksumMismatch`] on download failures.
    pub async fn install_vanilla(&self, version: &str) -> Result<VersionJson, MdtError> {
        let manifest = manifest::fetch_minecraft_version_manifest(&self.http, &self.mirror).await?;
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == version)
            .cloned()
            .ok_or_else(|| MdtError::VersionNotFound(version.to_string()))?;

        let urls = ManifestUrls::try_new(&self.mirror)?;
        let version_url = urls.apply_mirror(&entry.url, &self.mirror);
        let text = self
            .http
            .get(&version_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let version_json: VersionJson = serde_json::from_str(&text).map_err(|e| {
            MdtError::JsonParseError(e).context(format!("parsing version JSON for {version}"))
        })?;

        // Save the vanilla version JSON.
        let vdir = self.root.join("versions").join(&version_json.id);
        tokio::fs::create_dir_all(&vdir).await?;
        tokio::fs::write(vdir.join(format!("{}.json", version_json.id)), &text).await?;

        // Client JAR + libraries (+ asset index). Vanilla stays untouched.
        let tasks = downloader::build_tasks_from_version_json(&version_json, &self.root);
        downloader::execute_downloads(&self.http, tasks, self.concurrency, &self.mirror, &urls).await?;

        // Assets.
        if let Some(asset_index) = &version_json.asset_index {
            let asset_index_path = self
                .root
                .join("assets")
                .join("indexes")
                .join(format!("{}.json", asset_index.id));
            let content = tokio::fs::read_to_string(&asset_index_path).await?;
            let index: AssetIndex = serde_json::from_str(&content)?;
            let asset_tasks = downloader::build_tasks_from_asset_index(&index, &self.root);
            downloader::execute_downloads(&self.http, asset_tasks, self.concurrency, &self.mirror, &urls).await?;
        }

        Ok(version_json)
    }

    /// Install a mod loader into an isolated version directory.
    ///
    /// Uses the `[mc_version]+[Loader][LoaderVersion]` layout (see [`installer::loader_version_id`]),
    /// so vanilla `versions/<mc_version>` is never overwritten.
    ///
    /// # Errors
    ///
    /// * [`MdtError::LoaderInstallFailed`] if the loader doesn't support `mc_version`.
    /// * [`MdtError::JavaNotFound`] if a Method A loader is requested but there's no Java.
    pub async fn install_modloader(
        &self,
        mc_version: &str,
        loader: ModLoader,
        loader_version: &str,
    ) -> Result<InstalledVersion, MdtError> {
        if !loader.compatible_with(mc_version) {
            return Err(MdtError::LoaderInstallFailed(format!(
                "{} is not compatible with Minecraft {}",
                loader.display_name(),
                mc_version
            )));
        }

        let req = InstallRequest {
            mc_version: mc_version.to_string(),
            loader,
            loader_version: loader_version.to_string(),
            root_dir: self.root.clone(),
            mirror: self.mirror.clone(),
        };
        installer::install_modloader(&self.http, req).await
    }
}

/// Builder for [`MdtClient`].
#[derive(Debug)]
pub struct MdtClientBuilder {
    mirror: Mirror,
    root: PathBuf,
    concurrency: usize,
}

impl MdtClientBuilder {
    /// Start building with defaults ([`Mirror::BMCLAPI`], root `.`, 3 concurrent downloads).
    #[must_use]
    pub fn new() -> Self {
        Self {
            mirror: Mirror::default(),
            root: PathBuf::from("."),
            concurrency: DEFAULT_CONCURRENCY,
        }
    }

    /// Pick a mirror (default: [`Mirror::BMCLAPI`]).
    #[must_use]
    pub fn mirror(mut self, mirror: Mirror) -> Self {
        self.mirror = mirror;
        self
    }

    /// Set the target `.minecraft` root directory (default: `.`).
    #[must_use]
    pub fn root_dir(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    /// Override the download concurrency (clamped to at least 1; default 3).
    #[must_use]
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Finish building: creates the root directory and the shared HTTP client.
    pub fn build(self) -> Result<MdtClient, MdtError> {
        std::fs::create_dir_all(&self.root)?;
        let http = http::build_client()?;
        Ok(MdtClient {
            http,
            mirror: self.mirror,
            root: self.root,
            concurrency: self.concurrency,
        })
    }
}

impl Default for MdtClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_to_three_threads() {
        let client = MdtClient::builder().build().unwrap();
        assert_eq!(client.concurrency(), 3);
    }

    #[test]
    fn builder_concurrency_is_clamped_to_at_least_one() {
        let client = MdtClient::builder().concurrency(0).build().unwrap();
        assert_eq!(client.concurrency(), 1);
    }

    #[test]
    fn builder_concurrency_can_be_overridden() {
        let client = MdtClient::builder().concurrency(8).build().unwrap();
        assert_eq!(client.concurrency(), 8);
    }

    #[test]
    fn set_concurrency_updates_at_runtime() {
        let mut client = MdtClient::builder().build().unwrap();
        client.set_concurrency(6);
        assert_eq!(client.concurrency(), 6);
    }

    #[test]
    fn set_concurrency_clamps_to_at_least_one() {
        let mut client = MdtClient::builder().build().unwrap();
        client.set_concurrency(0);
        assert_eq!(client.concurrency(), 1);
    }
}
