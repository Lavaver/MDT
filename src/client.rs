//! The central [`MdtClient`] and its builder.

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
const DEFAULT_CONCURRENCY: usize = 32;

/// The central, plug-and-play MDT state container.
///
/// It owns the shared HTTP client (with connection pooling), the active [`Mirror`], and the
/// target `.minecraft` root directory. Build one with [`MdtClient::builder`] and reuse it across
/// as many installs as you like.
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

    /// The shared HTTP client (connection-pooled, browser User-Agent).
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

    /// Fetch the list of available Minecraft versions.
    pub async fn versions(&self) -> Result<Vec<VersionEntry>, MdtError> {
        let manifest = manifest::fetch_minecraft_version_manifest(&self.http, &self.mirror).await?;
        Ok(manifest.versions)
    }

    /// Download and install a vanilla Minecraft version.
    ///
    /// Returns the parsed [`VersionJson`] for the installed version. Handles library rules,
    /// natives extraction, the client JAR, and assets. The version JSON is persisted to
    /// `versions/<id>/<id>.json` so launchers and loader installers can read it.
    ///
    /// # Errors
    ///
    /// * [`MdtError::VersionNotFound`] if `version` is not in the manifest.
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

        // Persist the vanilla version JSON.
        let vdir = self.root.join("versions").join(&version_json.id);
        tokio::fs::create_dir_all(&vdir).await?;
        tokio::fs::write(vdir.join(format!("{}.json", version_json.id)), &text).await?;

        // Client JAR + libraries (+ asset index). The vanilla directory is left pristine.
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
    /// Enforces the `[mc_version]+[Loader][LoaderVersion]` isolation convention (see
    /// [`installer::loader_version_id`]), so the vanilla `versions/<mc_version>` directory is
    /// never overwritten.
    ///
    /// # Errors
    ///
    /// * [`MdtError::LoaderInstallFailed`] if the loader is incompatible with `mc_version`.
    /// * [`MdtError::JavaNotFound`] if a Method A loader is requested but no Java runtime exists.
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
    /// Start building with defaults ([`Mirror::BMCLAPI`], root `.`, 32 concurrent downloads).
    #[must_use]
    pub fn new() -> Self {
        Self {
            mirror: Mirror::default(),
            root: PathBuf::from("."),
            concurrency: DEFAULT_CONCURRENCY,
        }
    }

    /// Select the mirror source (default: [`Mirror::BMCLAPI`]).
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

    /// Override the download concurrency (clamped to at least 1; default 32).
    #[must_use]
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Finalize the builder, creating the target root directory and the shared HTTP client.
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
