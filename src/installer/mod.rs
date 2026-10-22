//! Mod-loader installation.
//!
//! Two strategies are used:
//!
//! * **Method A (Java injection)** — download the official installer JAR and run it against a
//!   local Java runtime, injecting only the isolated version-directory path. Used by modern
//!   Forge and NeoForge.
//! * **Method B (manual unpack)** — treat the installer as a ZIP archive, deploy its libraries,
//!   and synthesize the Minecraft version JSON by hand. No Java required. Used by legacy Forge,
//!   Fabric, Quilt, and LiteLoader.
//!
//! # Isolation guarantee
//!
//! Every loader install lands in `versions/<mc_version>+<Loader><loader_version>` (see
//! [`loader_version_id`]). The vanilla `versions/<mc_version>` directory is never touched by a
//! loader install, so a broken loader can never corrupt the underlying game.

mod fabric;
mod forge;
mod installer;
mod liteloader;
mod neoforge;
mod quilt;
mod version_json;

use std::path::{Path, PathBuf};

use futures::StreamExt;

use crate::error::MdtError;
use crate::mirror::Mirror;
use crate::modloader::ModLoader;

pub use version_json::{
    apply_mirror_to_json, GeneratedArtifact, GeneratedLibrary, GeneratedLibraryDownloads,
    GeneratedVersionJson,
};

/// How a loader was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    /// Method A: official installer driven by a local Java runtime.
    JavaInjection,
    /// Method B: manual unpack / JSON synthesis, no Java required.
    ManualUnpack,
}

impl std::fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallMethod::JavaInjection => write!(f, "JavaInjection"),
            InstallMethod::ManualUnpack => write!(f, "ManualUnpack"),
        }
    }
}

/// A complete mod-loader installation request.
#[derive(Debug, Clone)]
pub struct InstallRequest {
    /// Target vanilla Minecraft version, e.g. `1.20.1`.
    pub mc_version: String,
    /// The loader to install.
    pub loader: ModLoader,
    /// Loader version, e.g. Forge `47.2.0` or Fabric `0.15.11`.
    pub loader_version: String,
    /// The `.minecraft` root directory.
    pub root_dir: PathBuf,
    /// The mirror to download from.
    pub mirror: Mirror,
}

/// The result of installing a mod loader.
#[derive(Debug, Clone)]
pub struct InstalledVersion {
    /// The isolated version ID, e.g. `1.20.1-forge-47.2.0`.
    pub id: String,
    /// Which strategy produced this install.
    pub method: InstallMethod,
    /// On-disk path of the generated version JSON.
    pub version_json: PathBuf,
    /// On-disk path of the core JAR, if one was produced (Method A may not produce one).
    pub jar: Option<PathBuf>,
}

/// Install a mod loader, dispatching to the per-loader implementation.
pub async fn install_modloader(
    http: &reqwest::Client,
    req: InstallRequest,
) -> Result<InstalledVersion, MdtError> {
    match req.loader {
        ModLoader::Forge => forge::install(http, req).await,
        ModLoader::Neoforge => neoforge::install(http, req).await,
        ModLoader::Fabric => fabric::install(http, req).await,
        ModLoader::Quilt => quilt::install(http, req).await,
        ModLoader::Liteloader => liteloader::install(http, req).await,
    }
}

// ===== shared helpers =====

/// Compute the isolated version ID for Method B: `[mc_version]+[Loader][LoaderVersion]`.
///
/// For example `1.16.5+Fabric0.20.0` or `1.20.1+Forge47.2.0`. This ID is used as the
/// `versions/<id>` directory name, the version JSON filename, and the JSON's internal `id`
/// field — keeping the loader completely isolated from the vanilla `versions/<mc_version>`.
#[must_use]
pub fn loader_version_id(mc_version: &str, loader: ModLoader, loader_version: &str) -> String {
    format!("{mc_version}+{}{}", loader.display_name(), loader_version)
}

/// Compute the version ID produced by the official installer (Method A: Forge / NeoForge).
///
/// These IDs must match what the official installer writes on disk (e.g.
/// `1.20.1-forge-47.2.0`, `1.20.1-neoforge-47.1.106`) so the result can be verified.
#[must_use]
pub fn installer_version_id(mc_version: &str, loader: ModLoader, loader_version: &str) -> String {
    match loader {
        ModLoader::Forge => format!("{mc_version}-forge-{loader_version}"),
        ModLoader::Neoforge => format!("{mc_version}-neoforge-{loader_version}"),
        _ => loader_version_id(mc_version, loader, loader_version),
    }
}

/// Convert a Maven coordinate into a repository-relative path.
///
/// `classifier` (optional) yields names such as `forge-1.20.1-47.2.0-installer.jar`.
#[must_use]
pub fn maven_artifact_path(
    group: &str,
    artifact: &str,
    version: &str,
    classifier: Option<&str>,
) -> String {
    let group_path = group.replace('.', "/");
    let filename = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    format!("{group_path}/{artifact}/{version}/{filename}")
}

/// Derive a Maven repository-relative path from a `group:artifact:version` coordinate.
pub fn library_path_from_name(name: &str) -> Result<String, MdtError> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return Err(MdtError::LoaderInstallFailed(format!(
            "invalid Maven coordinate: {name}"
        )));
    }
    Ok(maven_artifact_path(parts[0], parts[1], parts[2], None))
}

/// Detect a local Java runtime, returning the path to its executable.
pub async fn detect_java() -> Result<PathBuf, MdtError> {
    // Fast path: JAVA_HOME.
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let p = PathBuf::from(home)
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        if p.is_file() {
            return Ok(p);
        }
    }

    // Fall back to PATH (`java -version` is a blocking probe, so run it off the runtime).
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let exe_name = exe.to_string();
    let found = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&exe_name).arg("-version").output().is_ok()
    })
    .await?;

    if found {
        Ok(PathBuf::from(exe))
    } else {
        Err(MdtError::JavaNotFound)
    }
}

/// Stream a URL to a file without checksum verification (for loader installer JARs).
pub async fn download_to_file(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), MdtError> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let resp = http.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    Ok(())
}

/// Extract a JAR (ZIP) to a directory, skipping directories and `META-INF/`, with zip-slip
/// protection.
pub async fn unzip_jar(src: &Path, dest: &Path) -> Result<(), MdtError> {
    let src = src.to_path_buf();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<(), MdtError> {
        std::fs::create_dir_all(&dest)?;
        let file = std::fs::File::open(&src)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name()?.into_owned();
            if entry.is_dir() || name.starts_with("META-INF/") {
                continue;
            }
            // zip-slip protection: reject path components that traverse upward.
            if name.split(['/', '\\']).any(|c| c == "..") {
                return Err(MdtError::LoaderInstallFailed(format!(
                    "refusing to extract unsafe path: {name}"
                )));
            }
            let out_path = dest.join(&name);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
        Ok(())
    })
    .await?
}

/// Recursively copy a directory (used to flatten an installer's `maven/` into `libraries/`).
pub async fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), MdtError> {
    let src = src.to_path_buf();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || copy_dir_sync(&src, &dest)).await?
}

fn copy_dir_sync(src: &Path, dest: &Path) -> Result<(), MdtError> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_sync(&entry.path(), &to)?;
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Ensure a directory exists and is empty (used for temporary directories).
pub async fn ensure_empty_dir(path: &Path) -> Result<(), MdtError> {
    if path.exists() {
        tokio::fs::remove_dir_all(path).await?;
    }
    tokio::fs::create_dir_all(path).await?;
    Ok(())
}

/// The version directory: `.minecraft/versions/<id>`.
#[must_use]
pub fn version_dir(root: &Path, id: &str) -> PathBuf {
    root.join("versions").join(id)
}

/// The global libraries directory: `.minecraft/libraries`.
#[must_use]
pub fn libraries_dir(root: &Path) -> PathBuf {
    root.join("libraries")
}
