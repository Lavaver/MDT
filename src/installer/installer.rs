//! Method A: official installer driven by a local Java runtime.
//!
//! MDT only downloads the official installer and injects the isolated version-directory path;
//! the official installer performs dependency resolution, extraction, and version-config
//! generation itself.

use std::path::Path;

use crate::error::MdtError;
use crate::mirror::Mirror;
use crate::urls::ManifestUrls;

use super::{detect_java, download_to_file, ensure_empty_dir};

/// Run the full Method A flow: download installer → detect Java → run natively → clean up.
pub async fn install_via_official_installer(
    http: &reqwest::Client,
    installer_url: &str,
    root_dir: &Path,
    mirror: &Mirror,
    urls: &ManifestUrls,
) -> Result<(), MdtError> {
    let temp_dir = root_dir.join(".mdt-installer-tmp");
    ensure_empty_dir(&temp_dir).await?;

    let installer_jar = temp_dir.join("installer.jar");
    let real_url = urls.apply_maven_mirror(installer_url, mirror);
    download_to_file(http, &real_url, &installer_jar).await?;

    let java = detect_java().await?;
    run_java_installer(&java, &installer_jar, root_dir).await?;

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    Ok(())
}

/// Invoke the local Java runtime to run the official installer.
pub async fn run_java_installer(
    java: &Path,
    installer: &Path,
    root_dir: &Path,
) -> Result<(), MdtError> {
    let java = java.to_path_buf();
    let installer = installer.to_path_buf();
    let root = root_dir.to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<(), MdtError> {
        let output = std::process::Command::new(&java)
            .arg("-jar")
            .arg(&installer)
            .arg("--installClient")
            .arg(&root)
            .output()
            .map_err(|e| {
                MdtError::IoError(e).context(format!("failed to spawn {}", installer.display()))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = [stderr.trim(), stdout.trim()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            let code = match output.status.code() {
                Some(c) => c.to_string(),
                None => "unknown".to_string(),
            };
            let message = if detail.is_empty() {
                format!("official installer exited with status {code}")
            } else {
                format!("official installer exited with status {code}: {detail}")
            };
            return Err(MdtError::LoaderInstallFailed(message));
        }
        Ok(())
    })
    .await?
}
