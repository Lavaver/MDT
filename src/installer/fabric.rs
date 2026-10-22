//! Fabric loader installation (Method B: manual unpack).
//!
//! Fabric ships only its core components (no official installer); MDT fetches the official
//! profile JSON, downloads its dependencies, and synthesizes the version JSON — no Java required.

use crate::error::MdtError;
use crate::urls::ManifestUrls;

use super::version_json::install_from_profile;
use super::{InstallRequest, InstalledVersion};

pub async fn install(
    http: &reqwest::Client,
    req: InstallRequest,
) -> Result<InstalledVersion, MdtError> {
    let urls = ManifestUrls::new(req.mirror);
    let profile_url = urls.fabric_profile(&req.loader_version, &req.mc_version);
    install_from_profile(http, &profile_url, &req, &urls).await
}
