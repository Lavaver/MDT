//! Shared HTTP client construction.

use std::time::Duration;

use crate::error::MdtError;

/// A Chrome-like `User-Agent`, used to avoid being blocked by mirror CDNs and WAFs that
/// reject requesters without a browser UA.
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// Build the shared HTTP client used by [`crate::MdtClient`].
///
/// Cheap to clone (`Arc` under the hood) and reuses one connection pool across all downloads,
/// so you're not doing a TLS handshake per file.
///
/// Timeouts protect against a stalled mirror without killing slow-but-healthy downloads: the
/// connect phase is capped at 15s, and a read timeout of 30s aborts only when no bytes arrive for
/// 30s (it resets after every successful read). There is deliberately no *total* request timeout,
/// since large files legitimately take longer than any fixed wall-clock bound.
pub fn build_client() -> Result<reqwest::Client, MdtError> {
    Ok(reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .pool_idle_timeout(Duration::from_secs(90))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_client_succeeds() {
        let _client = build_client().unwrap();
    }

    #[test]
    fn user_agent_is_browser_like() {
        assert!(USER_AGENT.contains("Mozilla"), "missing Mozilla token");
        assert!(USER_AGENT.contains("Chrome"), "missing Chrome token");
    }
}
