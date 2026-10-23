//! The single, strict error type for the whole crate.
//!
//! Every fallible operation in this crate returns [`Result<T, MdtError>`][std::result::Result]
//! so downstream launchers can pattern-match on the exact failure instead of handling a
//! catch-all boxed error.

use thiserror::Error;

/// The error type produced by every fallible MDT operation.
///
/// Variants fall into two families:
///
/// * **Transparent passthroughs** (`#[from]`) for foreign error types — network, JSON, XML,
///   I/O, ZIP, and task joins. These let call sites use `?` directly.
/// * **Structural failures** produced by MDT itself — version not found, checksum mismatch,
///   loader incompatibility, and so on.
///
/// [`MdtError::context`] wraps any variant with human-readable context, keeping the same
/// ergonomics as `anyhow` while remaining a strict, matchable enum.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MdtError {
    /// An HTTP request failed (DNS, TLS, status code, streaming, ...).
    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// A payload could not be deserialized as JSON.
    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    /// A payload could not be deserialized as XML (Maven metadata).
    #[error("XML parse error: {0}")]
    XmlParseError(#[from] quick_xml::de::DeError),

    /// A filesystem operation failed.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// A ZIP archive could not be opened or extracted.
    #[error("ZIP error: {0}")]
    ZipError(#[from] zip::result::ZipError),

    /// A background `spawn_blocking` task panicked or was cancelled.
    #[error("background task failed: {0}")]
    TaskPanic(#[from] tokio::task::JoinError),

    /// An error wrapped with additional human-readable context.
    #[error("{context}: {source}")]
    Context {
        /// A description of where the failure occurred.
        context: String,
        /// The underlying error.
        #[source]
        source: Box<MdtError>,
    },

    /// The requested Minecraft version does not exist in the version manifest.
    #[error("version not found: {0}")]
    VersionNotFound(String),

    /// A mod loader could not be installed.
    #[error("loader install failed: {0}")]
    LoaderInstallFailed(String),

    /// No Java runtime was found on this machine.
    #[error("Java runtime not found: install Java 8+ or 17+ and retry")]
    JavaNotFound,

    /// A downloaded file failed its SHA-1 integrity check.
    #[error("checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// The destination path of the corrupted file.
        path: String,
        /// The expected SHA-1 (from the manifest).
        expected: String,
        /// The SHA-1 actually computed over the downloaded bytes.
        actual: String,
    },

    /// A manifest or version document was structurally invalid.
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    /// A mirror provider name is not declared in `providers.toml`.
    #[error("unknown mirror provider: {0}")]
    UnknownProvider(String),
}

impl MdtError {
    /// Wrap this error with human-readable context, returning [`MdtError::Context`].
    ///
    /// # Example
    ///
    /// ```
    /// use mdt::MdtError;
    ///
    /// let wrapped = MdtError::JavaNotFound.context("installing Forge 1.20.1");
    /// assert_eq!(
    ///     wrapped.to_string(),
    ///     "installing Forge 1.20.1: Java runtime not found: install Java 8+ or 17+ and retry"
    /// );
    /// ```
    #[must_use]
    pub fn context(self, context: impl Into<String>) -> Self {
        MdtError::Context {
            context: context.into(),
            source: Box::new(self),
        }
    }
}
