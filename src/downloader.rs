//! Game-file download and SHA-1 verification.
//!
//! Parses `version.json`, builds concurrent download tasks, verifies every file against its
//! manifest SHA-1, applies mirror rewriting, filters library rules, and extracts natives.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::MdtError;
use crate::mirror::Mirror;
use crate::urls::ManifestUrls;

// ===== 1. data structures =====

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: LibraryDownloads,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// The parsed Minecraft `version.json` for a single version.
///
/// Returned by [`crate::MdtClient::install_vanilla`] so launchers can inspect the installed
/// version's assets, libraries, and downloads.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndexInfo>,
    pub downloads: Option<HashMap<String, Artifact>>,
    #[serde(default)]
    pub libraries: Vec<Library>,
}

#[derive(Deserialize, Debug)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Debug)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

// ===== 2. download tasks =====

/// A single file to download and verify.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest_path: PathBuf,
    pub sha1: String,
    pub size: u64,
    /// If set, the downloaded archive is extracted here (used for natives).
    pub extract_to: Option<PathBuf>,
}

// ===== 3. rule evaluation =====

/// Evaluate whether the current platform should download this library.
fn evaluate_rules(rules: &Option<Vec<Rule>>) -> bool {
    let Some(rules) = rules else {
        return true;
    };

    let mut allowed = false;
    let os_name = std::env::consts::OS;
    let os_arch = std::env::consts::ARCH;

    for rule in rules {
        let mut matches = true;

        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                let name_match = match name.as_str() {
                    "windows" => os_name == "windows",
                    "osx" => os_name == "macos",
                    "linux" => os_name == "linux",
                    _ => false,
                };
                if !name_match {
                    matches = false;
                }
            }
            if let Some(arch) = &os.arch {
                let arch_match = match arch.as_str() {
                    "x86" => os_arch == "x86",
                    "x86_64" => os_arch == "x86_64",
                    "arm64" => os_arch == "aarch64",
                    _ => false,
                };
                if !arch_match {
                    matches = false;
                }
            }
        }

        // Features are not evaluated at download time; an unresolved feature means no match.
        if rule.features.is_some() {
            matches = false;
        }

        if matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

// ===== 4. download execution =====

/// Execute a batch of download tasks concurrently, verifying each against its SHA-1.
///
/// An empty task queue is treated as an error rather than a silent success.
pub async fn execute_downloads(
    http: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    mirror: Mirror,
    urls: &ManifestUrls,
) -> Result<(), MdtError> {
    if tasks.is_empty() {
        return Err(MdtError::InvalidManifest(
            "download task queue is empty: the version manifest may be malformed".to_string(),
        ));
    }

    let concurrency = concurrency.max(1);
    let results = stream::iter(
        tasks
            .into_iter()
            .map(|task| download_single_file(task, mirror, urls, http)),
    )
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;

    for res in results {
        res?;
    }
    Ok(())
}

/// Stream a file in chunks and compute its SHA-1, avoiding loading the whole file into memory.
async fn sha1_file(path: &Path) -> Result<String, MdtError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| MdtError::IoError(e).context(format!("opening {}", path.display())))?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 8192];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Download a single file and verify its SHA-1 (no pre-check, no natives extraction).
async fn download_file_stream(
    task: &DownloadTask,
    mirror: Mirror,
    urls: &ManifestUrls,
    http: &reqwest::Client,
) -> Result<(), MdtError> {
    let real_url = urls.apply_mirror(&task.url, mirror);
    let resp = http.get(&real_url).send().await?.error_for_status()?;

    let mut file = tokio::fs::File::create(&task.dest_path).await?;
    let mut stream = resp.bytes_stream();
    let mut hasher = Sha1::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    let actual = hex::encode(hasher.finalize());
    if actual != task.sha1 {
        let _ = tokio::fs::remove_file(&task.dest_path).await;
        return Err(MdtError::ChecksumMismatch {
            path: task.dest_path.display().to_string(),
            expected: task.sha1.clone(),
            actual,
        });
    }
    Ok(())
}

/// Download a single file, skipping if it already exists with a matching SHA-1, then extract
/// natives when requested.
async fn download_single_file(
    task: DownloadTask,
    mirror: Mirror,
    urls: &ManifestUrls,
    http: &reqwest::Client,
) -> Result<(), MdtError> {
    // 1. Pre-check: skip if the file already exists and its SHA-1 matches.
    let mut need_download = true;
    if task.dest_path.exists() {
        let meta = tokio::fs::metadata(&task.dest_path).await?;
        if meta.len() == task.size {
            let hash = sha1_file(&task.dest_path).await?;
            if hash == task.sha1 {
                need_download = false;
            }
        }
        if need_download {
            tokio::fs::remove_file(&task.dest_path).await?;
        }
    }

    // 2. Download if necessary.
    if need_download {
        if let Some(parent) = task.dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        download_file_stream(&task, mirror, urls, http).await?;
    }

    // 3. Extract natives if requested.
    if let Some(extract_dir) = task.extract_to {
        let dest_path = task.dest_path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), MdtError> {
            let file = std::fs::File::open(&dest_path)
                .map_err(|e| MdtError::IoError(e).context(format!("opening natives jar {}", dest_path.display())))?;
            let mut archive = zip::ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)?;
                let name = entry.name()?.into_owned();

                if entry.is_dir() || name.starts_with("META-INF/") {
                    continue;
                }

                let out_path = extract_dir.join(&name);
                if let Some(p) = out_path.parent() {
                    std::fs::create_dir_all(p)?;
                }

                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
            }
            Ok(())
        })
        .await??;
    }

    Ok(())
}

// ===== 5. task generators =====

/// Build download tasks from a parsed version JSON (client JAR, libraries, asset index).
pub fn build_tasks_from_version_json(version_json: &VersionJson, root_dir: &Path) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();

    // 1. client.jar
    if let Some(downloads) = &version_json.downloads {
        if let Some(client) = downloads.get("client") {
            tasks.push(DownloadTask {
                url: client.url.clone(),
                dest_path: root_dir
                    .join("versions")
                    .join(&version_json.id)
                    .join(format!("{}.jar", version_json.id)),
                sha1: client.sha1.clone(),
                size: client.size,
                extract_to: None,
            });
        }
    }

    // 2. libraries
    let natives_dir = root_dir
        .join("versions")
        .join(&version_json.id)
        .join(format!("{}-natives", version_json.id));
    let os_name = std::env::consts::OS;

    for lib in &version_json.libraries {
        if !evaluate_rules(&lib.rules) {
            continue;
        }

        if let Some(artifact) = &lib.downloads.artifact {
            if let Some(path) = &artifact.path {
                tasks.push(DownloadTask {
                    url: artifact.url.clone(),
                    dest_path: root_dir.join("libraries").join(path),
                    sha1: artifact.sha1.clone(),
                    size: artifact.size,
                    extract_to: None,
                });
            }
        }

        if let Some(classifiers) = &lib.downloads.classifiers {
            let native_key = match os_name {
                "windows" => "natives-windows",
                "macos" => "natives-macos",
                "linux" => "natives-linux",
                _ => "",
            };

            let native_artifact = classifiers
                .iter()
                .find(|(key, _)| key.starts_with(native_key))
                .map(|(_, art)| art);

            if let Some(art) = native_artifact {
                if let Some(path) = &art.path {
                    tasks.push(DownloadTask {
                        url: art.url.clone(),
                        dest_path: root_dir.join("libraries").join(path),
                        sha1: art.sha1.clone(),
                        size: art.size,
                        extract_to: Some(natives_dir.clone()),
                    });
                }
            }
        }
    }

    // 3. asset index
    if let Some(asset_index) = &version_json.asset_index {
        tasks.push(DownloadTask {
            url: asset_index.url.clone(),
            dest_path: root_dir
                .join("assets")
                .join("indexes")
                .join(format!("{}.json", asset_index.id)),
            sha1: asset_index.sha1.clone(),
            size: asset_index.size,
            extract_to: None,
        });
    }

    tasks
}

/// Build download tasks for every asset object in an asset index.
pub fn build_tasks_from_asset_index(asset_index: &AssetIndex, root_dir: &Path) -> Vec<DownloadTask> {
    asset_index
        .objects
        .values()
        .map(|object| {
            let prefix = &object.hash[0..2];
            DownloadTask {
                url: format!("https://resources.download.minecraft.net/{}/{}", prefix, object.hash),
                dest_path: root_dir
                    .join("assets")
                    .join("objects")
                    .join(prefix)
                    .join(&object.hash),
                sha1: object.hash.clone(),
                size: object.size,
                extract_to: None,
            }
        })
        .collect()
}
