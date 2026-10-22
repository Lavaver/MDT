//! Quilt loader installation (Method B: manual unpack).
//!
//! Quilt ships only its core components; MDT fetches the official profile JSON, downloads its
//! dependencies, and synthesizes the version JSON — no Java required. Under [`crate::Mirror::BMCLAPI`]
//! the profile and Maven URLs fall back to the official Quilt servers (BMCLAPI does not proxy them).

use crate::error::MdtError;
use crate::urls::ManifestUrls;

use super::version_json::install_from_profile;
use super::{InstallRequest, InstalledVersion};

pub async fn install(
    http: &reqwest::Client,
    req: InstallRequest,
) -> Result<InstalledVersion, MdtError> {
    let urls = ManifestUrls::new(req.mirror);
    let profile_url = urls.quilt_profile_url(&req.loader_version, &req.mc_version, req.mirror);
    install_from_profile(http, &profile_url, &req, &urls).await
}
