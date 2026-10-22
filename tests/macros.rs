//! Tests for the exported declarative macros.

use mdt::{download_task, mdt_install, version_dir, MdtClient, Mirror, ModLoader};

#[test]
fn version_dir_macro_produces_isolated_path() {
    let dir = std::env::temp_dir().join("mdt-macro-vdir");
    let client = MdtClient::builder()
        .mirror(Mirror::BMCLAPI)
        .root_dir(&dir)
        .build()
        .unwrap();

    let path = version_dir!(client, "1.16.5", Fabric, "0.20.0");
    assert!(path.ends_with("versions/1.16.5+Fabric0.20.0"));

    // The bare loader identifier maps to the corresponding `ModLoader` variant.
    let path = version_dir!(client, "1.20.1", Neoforge, "47.1.106");
    assert!(path.ends_with("versions/1.20.1+NeoForge47.1.106"));
}

#[test]
fn download_task_macro_defaults() {
    let task = download_task!("https://example.com/a.jar", "libs/a.jar", "abc123");
    assert_eq!(task.url, "https://example.com/a.jar");
    assert_eq!(task.sha1, "abc123");
    assert_eq!(task.size, 0);
    assert_eq!(task.dest_path, std::path::PathBuf::from("libs/a.jar"));
    assert!(task.extract_to.is_none());
}

#[test]
fn download_task_macro_with_size_and_extract() {
    let task = download_task!(
        "https://example.com/a.jar",
        "libs/a.jar",
        "abc123",
        42,
        extract_to = "natives"
    );
    assert_eq!(task.size, 42);
    assert_eq!(task.extract_to, Some(std::path::PathBuf::from("natives")));
}

#[test]
fn mdt_install_macro_expands_to_method_call() {
    let dir = std::env::temp_dir().join("mdt-macro-install");
    let client = MdtClient::builder()
        .mirror(Mirror::BMCLAPI)
        .root_dir(&dir)
        .build()
        .unwrap();

    // Only check that the macro expands and type-checks; no network, no await.
    let _future = mdt_install!(client, "1.16.5", ModLoader::Fabric, "0.20.0");
}
