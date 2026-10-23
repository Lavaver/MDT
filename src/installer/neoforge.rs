//! NeoForge loader installation (Method A: modern injection).

use crate::error::MdtError;
use crate::modloader::ModLoader;
use crate::urls::ManifestUrls;

use super::installer::install_via_official_installer;
use super::{installer_version_id, version_dir, InstallMethod, InstallRequest, InstalledVersion};

pub async fn install(
    http: &reqwest::Client,
    req: InstallRequest,
) -> Result<InstalledVersion, MdtError> {
    let urls = ManifestUrls::try_new(&req.mirror)?;
    let id = installer_version_id(&req.mc_version, ModLoader::Neoforge, &req.loader_version);

    // NeoForge's Maven version is the loader version itself (e.g. `47.1.106`), no MC prefix.
    let maven_version = req.loader_version.clone();
    let installer_url = format!(
        "{}/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        urls.neoforge_maven, maven_version, maven_version
    );

    install_via_official_installer(http, &installer_url, &req.root_dir, &req.mirror, &urls).await?;

    // Verify: the official installer should produce a version JSON.
    let vdir = version_dir(&req.root_dir, &id);
    let json_path = vdir.join(format!("{id}.json"));
    if !json_path.exists() {
        return Err(MdtError::LoaderInstallFailed(format!(
            "official installer finished but produced no version JSON at {}",
            json_path.display()
        )));
    }

    let jar_path = vdir.join(format!("{id}.jar"));
    let jar = if jar_path.exists() {
        Some(jar_path)
    } else {
        None
    };

    Ok(InstalledVersion {
        id,
        method: InstallMethod::JavaInjection,
        version_json: json_path,
        jar,
    })
}
