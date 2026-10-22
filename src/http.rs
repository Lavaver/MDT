//! Shared HTTP client construction.

use std::time::Duration;

use crate::error::MdtError;

/// A Chrome-like `User-Agent`, used to avoid being blocked by mirror CDNs and WAFs that
/// reject requesters without a browser UA.
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// Build the shared HTTP client used by [`crate::MdtClient`].
///
/// The returned client is cheap to clone (it is backed by an `Arc` internally) and reuses a
/// single connection pool across every concurrent download task, avoiding per-file TLS
/// handshakes.
pub fn build_client() -> Result<reqwest::Client, MdtError> {
    Ok(reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .pool_idle_timeout(Duration::from_secs(90))
        .build()?)
}
