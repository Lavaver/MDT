//! Declarative macros for ergonomic construction.
//!
//! These are exported at the crate root (via `#[macro_export]`) and are available to downstream
//! launchers as `mdt::version_dir!`, `mdt::download_task!`, and `mdt::mdt_install!`. All
//! internal references use `$crate::` so the macros are hygienic when used from other crates.

/// Build the isolated version directory for a loader install.
///
/// Expands to `<client.root_dir()>/versions/<mc_version>+<Loader><loader_version>`, enforcing the
/// isolation convention in a single expression. The loader is passed as a bare variant name
/// (`Fabric`, `Forge`, `Quilt`, `Neoforge`, `Liteloader`).
///
/// ```
/// use mdt::{version_dir, MdtClient, Mirror};
///
/// let dir = std::env::temp_dir().join("mdt-version-dir-doctest");
/// let client = MdtClient::builder()
///     .mirror(Mirror::BMCLAPI)
///     .root_dir(&dir)
///     .build()
///     .unwrap();
///
/// let path = version_dir!(client, "1.16.5", Fabric, "0.20.0");
/// assert!(path.ends_with("versions/1.16.5+Fabric0.20.0"));
/// ```
#[macro_export]
macro_rules! version_dir {
    ($client:expr, $mc_version:expr, $loader:ident, $loader_version:expr) => {
        $crate::installer::version_dir(
            $client.root_dir(),
            &$crate::installer::loader_version_id(
                $mc_version,
                $crate::ModLoader::$loader,
                $loader_version,
            ),
        )
    };
}

/// Construct a [`DownloadTask`][crate::DownloadTask] with minimal ceremony.
///
/// `size` defaults to `0` (unknown) and `extract_to` defaults to `None`; both can be supplied
/// explicitly via the longer forms.
///
/// ```
/// use mdt::download_task;
///
/// let task = download_task!("https://example.com/lib.jar", "libraries/lib.jar", "abc123", 42);
/// assert_eq!(task.url, "https://example.com/lib.jar");
/// assert_eq!(task.sha1, "abc123");
/// assert_eq!(task.size, 42);
/// assert!(task.extract_to.is_none());
/// ```
#[macro_export]
macro_rules! download_task {
    ($url:expr, $dest:expr, $sha1:expr $(,)?) => {
        $crate::DownloadTask {
            url: $url.into(),
            dest_path: ::std::path::PathBuf::from($dest),
            sha1: $sha1.into(),
            size: 0,
            extract_to: None,
        }
    };
    ($url:expr, $dest:expr, $sha1:expr, $size:expr $(,)?) => {
        $crate::DownloadTask {
            url: $url.into(),
            dest_path: ::std::path::PathBuf::from($dest),
            sha1: $sha1.into(),
            size: $size,
            extract_to: None,
        }
    };
    ($url:expr, $dest:expr, $sha1:expr, $size:expr, extract_to = $extract:expr $(,)?) => {
        $crate::DownloadTask {
            url: $url.into(),
            dest_path: ::std::path::PathBuf::from($dest),
            sha1: $sha1.into(),
            size: $size,
            extract_to: Some(::std::path::PathBuf::from($extract)),
        }
    };
}

/// Install a mod loader in one line.
///
/// A thin, ergonomic wrapper around
/// [`MdtClient::install_modloader`][crate::MdtClient::install_modloader].
///
/// ```no_run
/// use mdt::{mdt_install, MdtClient, Mirror, ModLoader};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = MdtClient::builder().mirror(Mirror::BMCLAPI).root_dir("./.minecraft").build()?;
///     mdt_install!(client, "1.16.5", ModLoader::Fabric, "0.20.0").await?;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! mdt_install {
    ($client:expr, $mc_version:expr, $loader:expr, $loader_version:expr) => {
        $client.install_modloader($mc_version, $loader, $loader_version)
    };
}
