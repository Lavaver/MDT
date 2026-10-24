//! Tests for the new `install_modloader` / `uninstall_modloader` entry points.
//!
//! These cover the parts that run before any network access (version-JSON discovery and
//! compatibility checks), so they are safe to run offline. The download/verify paths are covered
//! by the existing wiremock-backed tests in `downloader.rs`.

use std::sync::atomic::{AtomicU64, Ordering};

use mdt::{MdtClient, MdtError, Mirror, ModLoader};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_dir(tag: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("mdt-install-{tag}-{}-{n}", std::process::id()))
}

fn client(root: &std::path::Path) -> MdtClient {
    MdtClient::builder()
        .mirror(Mirror::Mojang)
        .root_dir(root)
        .build()
        .unwrap()
}

#[tokio::test]
async fn install_modloader_fails_fast_when_version_json_is_missing() {
    let root = unique_dir("root");
    let vanilla_dir = unique_dir("vanilla");
    std::fs::create_dir_all(&vanilla_dir).unwrap();

    let err = client(&root)
        .install_modloader(&vanilla_dir, ModLoader::Fabric, "0.15.11", true)
        .await
        .unwrap_err();
    assert!(matches!(err, MdtError::InvalidManifest(_)), "got {err:?}");

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&vanilla_dir).ok();
}

#[tokio::test]
async fn install_modloader_rejects_incompatible_loader_before_network() {
    let root = unique_dir("root");
    let vanilla_dir = unique_dir("vanilla");
    std::fs::create_dir_all(&vanilla_dir).unwrap();
    let name = vanilla_dir.file_name().unwrap().to_str().unwrap().to_string();
    std::fs::write(vanilla_dir.join(format!("{name}.json")), r#"{"id": "1.12.2"}"#).unwrap();

    // Fabric does not support 1.12.2.
    let err = client(&root)
        .install_modloader(&vanilla_dir, ModLoader::Fabric, "0.15.11", false)
        .await
        .unwrap_err();
    assert!(matches!(err, MdtError::LoaderInstallFailed(_)), "got {err:?}");

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&vanilla_dir).ok();
}

#[tokio::test]
async fn uninstall_modloader_fails_fast_when_version_json_is_missing() {
    let root = unique_dir("root");
    let vanilla_dir = unique_dir("vanilla");
    std::fs::create_dir_all(&vanilla_dir).unwrap();

    let err = client(&root)
        .uninstall_modloader(&vanilla_dir, ModLoader::Fabric)
        .await
        .unwrap_err();
    assert!(matches!(err, MdtError::InvalidManifest(_)), "got {err:?}");

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&vanilla_dir).ok();
}
