//! Shorthand macros.
//!
//! Exported at the crate root as `mdt::version_dir!`, `mdt::download_task!`, and
//! `mdt::full_install!`. Internal refs use `$crate::` so they're hygienic from other crates.

/// The isolated version directory for a loader install.
///
/// Expands to `<client.root_dir()>/versions/<mc_version>+<Loader><loader_version>`. Pass the
/// loader as a bare variant name (`Fabric`, `Forge`, `Quilt`, `Neoforge`, `Liteloader`).
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

/// A [`DownloadTask`][crate::DownloadTask] in one line.
///
/// `size` defaults to `0` (unknown) and `extract_to` to `None`; the longer forms set them.
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
            label: None,
            by: None,
        }
    };
    ($url:expr, $dest:expr, $sha1:expr, $size:expr $(,)?) => {
        $crate::DownloadTask {
            url: $url.into(),
            dest_path: ::std::path::PathBuf::from($dest),
            sha1: $sha1.into(),
            size: $size,
            extract_to: None,
            label: None,
            by: None,
        }
    };
    ($url:expr, $dest:expr, $sha1:expr, $size:expr, extract_to = $extract:expr $(,)?) => {
        $crate::DownloadTask {
            url: $url.into(),
            dest_path: ::std::path::PathBuf::from($dest),
            sha1: $sha1.into(),
            size: $size,
            extract_to: Some(::std::path::PathBuf::from($extract)),
            label: None,
            by: None,
        }
    };
}

/// **FULL INSTALL** vanilla *and* a mod loader in one line.
///
/// Expands to [`MdtClient::full_install`][crate::MdtClient::full_install]: it installs the vanilla
/// version first (if it isn't already present), then the loader. Pass a trailing `false` to install
/// the loader in place instead of copy-on-write.
///
/// ```no_run
/// use mdt::{full_install, MdtClient, Mirror, ModLoader};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = MdtClient::builder().mirror(Mirror::BMCLAPI).root_dir("./.minecraft").build()?;
///     full_install!(client, "1.16.5", ModLoader::Fabric, "0.20.0").await?;         // vanilla + copy-on-write
///     full_install!(client, "1.16.5", ModLoader::Quilt, "0.24.0", false).await?;    // vanilla + in place
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! full_install {
    ($client:expr, $mc_version:expr, $loader:expr, $loader_version:expr) => {
        $client.full_install($mc_version, $loader, $loader_version, true)
    };
    ($client:expr, $mc_version:expr, $loader:expr, $loader_version:expr, $copy_vanilla:expr) => {
        $client.full_install($mc_version, $loader, $loader_version, $copy_vanilla)
    };
}
