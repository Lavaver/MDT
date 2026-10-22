//! Tests for mod-loader version IDs, mirror rewriting, and version parsing.

use std::collections::HashMap;

use mdt::installer::{
    apply_mirror_to_json, installer_version_id, library_path_from_name, loader_version_id,
    maven_artifact_path, GeneratedLibrary, GeneratedVersionJson,
};
use mdt::manifest::{
    loader_versions_from_data, neoforge_mc_prefix, FabricVersionsList, LatestMinecraftVersion,
    ManifestData, MavenMetadata, MavenVersions, MavenVersioning, MinecraftVersionsList,
    QuiltVersionsList,
};
use mdt::{ManifestUrls, McVersion, Mirror, ModLoader};

#[test]
fn test_loader_compatible_with() {
    // LiteLoader: 1.12.2 and earlier.
    assert!(ModLoader::Liteloader.compatible_with("1.12.2"));
    assert!(ModLoader::Liteloader.compatible_with("1.7.10"));
    assert!(!ModLoader::Liteloader.compatible_with("1.13"));
    assert!(!ModLoader::Liteloader.compatible_with("1.20.1"));

    // Fabric / Quilt: 1.14 and later.
    assert!(!ModLoader::Fabric.compatible_with("1.12.2"));
    assert!(ModLoader::Fabric.compatible_with("1.14"));
    assert!(ModLoader::Quilt.compatible_with("1.14.4"));
    assert!(ModLoader::Quilt.compatible_with("1.20.1"));

    // NeoForge: 1.21 and later.
    assert!(!ModLoader::Neoforge.compatible_with("1.20.1"));
    assert!(ModLoader::Neoforge.compatible_with("1.21"));
    assert!(ModLoader::Neoforge.compatible_with("1.21.1"));

    // Forge: unrestricted.
    assert!(ModLoader::Forge.compatible_with("1.7.10"));
    assert!(ModLoader::Forge.compatible_with("1.20.1"));

    // New-style year numbering (26.x onward): treated as modern.
    assert!(!ModLoader::Liteloader.compatible_with("26.3"));
    assert!(ModLoader::Fabric.compatible_with("26.3"));
    assert!(ModLoader::Quilt.compatible_with("26.3.1"));
    assert!(ModLoader::Neoforge.compatible_with("26.3"));
    assert!(ModLoader::Forge.compatible_with("26.3"));
}

#[test]
fn test_mc_version_parse() {
    // Old style.
    assert_eq!(McVersion::parse("1.12.2"), Some(McVersion::new(1, 12, 2)));
    assert_eq!(McVersion::parse("1.14"), Some(McVersion::new(1, 14, 0)));
    assert_eq!(McVersion::parse("1.21.1"), Some(McVersion::new(1, 21, 1)));
    // New-style year numbering (26.x onward).
    assert_eq!(McVersion::parse("26.3"), Some(McVersion::new(26, 3, 0)));
    assert_eq!(McVersion::parse("26.3.1"), Some(McVersion::new(26, 3, 1)));
    // Suffix stripping.
    assert_eq!(McVersion::parse("1.21-rc1"), Some(McVersion::new(1, 21, 0)));
    assert_eq!(McVersion::parse("26.3-rc-3"), Some(McVersion::new(26, 3, 0)));
    // Dotless snapshots are unparseable.
    assert_eq!(McVersion::parse("24w05a"), None);
    // New style > any old style.
    assert!(McVersion::parse("26.3").unwrap() > McVersion::parse("1.21.1").unwrap());
}

#[test]
fn test_loader_version_id() {
    assert_eq!(
        loader_version_id("1.16.5", ModLoader::Fabric, "0.20.0"),
        "1.16.5+Fabric0.20.0"
    );
    assert_eq!(
        loader_version_id("1.20.1", ModLoader::Forge, "47.2.0"),
        "1.20.1+Forge47.2.0"
    );
    assert_eq!(
        loader_version_id("1.20.1", ModLoader::Neoforge, "47.1.106"),
        "1.20.1+NeoForge47.1.106"
    );
    assert_eq!(
        loader_version_id("1.20.1", ModLoader::Quilt, "0.24.0"),
        "1.20.1+Quilt0.24.0"
    );
    assert_eq!(
        loader_version_id("1.12.2", ModLoader::Liteloader, "1.12.2"),
        "1.12.2+LiteLoader1.12.2"
    );
}

#[test]
fn test_installer_version_id() {
    assert_eq!(
        installer_version_id("1.20.1", ModLoader::Forge, "47.2.0"),
        "1.20.1-forge-47.2.0"
    );
    assert_eq!(
        installer_version_id("1.20.1", ModLoader::Neoforge, "47.1.106"),
        "1.20.1-neoforge-47.1.106"
    );
}

#[test]
fn test_maven_artifact_path() {
    assert_eq!(
        maven_artifact_path("net.minecraftforge", "forge", "1.20.1-47.2.0", Some("installer")),
        "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"
    );
    assert_eq!(
        maven_artifact_path("com.mumfrey", "liteloader", "1.12.2", None),
        "com/mumfrey/liteloader/1.12.2/liteloader-1.12.2.jar"
    );
}

#[test]
fn test_library_path_from_name() {
    assert_eq!(
        library_path_from_name("net.fabricmc:fabric-loader:0.15.11").unwrap(),
        "net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar"
    );
    assert!(library_path_from_name("bad").is_err());
    assert!(library_path_from_name("a:b").is_err());
}

#[test]
fn test_quilt_manifest_parse() {
    let json = r#"{
        "game": [{"version": "1.20.1", "stable": true}],
        "mappings": [
            {"maven": "org.quiltmc:quilt-mappings:1.20.1+build.1", "version": "1.20.1+build.1",
             "gameVersion": "1.20.1", "build": 1, "separator": "+build."}
        ],
        "hashed": [
            {"maven": "org.quiltmc:hashed:1.20.1", "version": "1.20.1"}
        ],
        "loader": [
            {"maven": "org.quiltmc:quilt-loader:0.24.0", "version": "0.24.0", "build": 2,
             "separator": "."}
        ],
        "installer": [
            {"maven": "org.quiltmc:quilt-installer:0.15.1", "version": "0.15.1", "url": "https://x"}
        ]
    }"#;

    let parsed: QuiltVersionsList = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.loader.len(), 1);
    assert_eq!(parsed.loader[0].version, "0.24.0");
}

#[test]
fn test_neoforge_mc_prefix() {
    assert_eq!(neoforge_mc_prefix("1.20.1"), Some("47.".to_string()));
    assert_eq!(neoforge_mc_prefix("1.20.2"), Some("48.".to_string()));
    assert_eq!(neoforge_mc_prefix("1.20.3"), Some("49.".to_string()));
    assert_eq!(neoforge_mc_prefix("1.20.4"), Some("20.4.".to_string()));
    assert_eq!(neoforge_mc_prefix("1.21.1"), Some("21.1.".to_string()));
    assert_eq!(neoforge_mc_prefix("1.21"), Some("21.".to_string()));
    // New-style year numbering (26.x onward): prefix preserved verbatim.
    assert_eq!(neoforge_mc_prefix("26.3"), Some("26.3.".to_string()));
    assert_eq!(neoforge_mc_prefix("26.3.1"), Some("26.3.1.".to_string()));
}

#[test]
fn test_apply_maven_mirror() {
    let urls = ManifestUrls::new(Mirror::BMCLAPI);
    const M: &str = "https://bmclapi2.bangbang93.com/maven";

    assert_eq!(
        urls.apply_maven_mirror("http://files.minecraftforge.net/maven/net/minecraftforge/forge/1.12.2-14.23.5.2859/forge-1.12.2-14.23.5.2859-universal.jar", Mirror::BMCLAPI),
        format!("{M}/net/minecraftforge/forge/1.12.2-14.23.5.2859/forge-1.12.2-14.23.5.2859-universal.jar")
    );
    assert_eq!(
        urls.apply_maven_mirror("https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json", Mirror::BMCLAPI),
        format!("{M}/net/minecraftforge/forge/maven-metadata.json")
    );
    assert_eq!(
        urls.apply_maven_mirror("https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar", Mirror::BMCLAPI),
        format!("{M}/net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar")
    );
    // Quilt Maven: BMCLAPI does not proxy org.quiltmc, so keep the official URL.
    assert_eq!(
        urls.apply_maven_mirror("https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-loader/0.24.0/quilt-loader-0.24.0.jar", Mirror::BMCLAPI),
        "https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-loader/0.24.0/quilt-loader-0.24.0.jar"
    );
    // Mojang source returns unchanged.
    let mojang = ManifestUrls::new(Mirror::Mojang);
    assert_eq!(
        mojang.apply_maven_mirror("http://files.minecraftforge.net/maven/x", Mirror::Mojang),
        "http://files.minecraftforge.net/maven/x"
    );
}

#[test]
fn test_generated_library_maven() {
    let lib = GeneratedLibrary::maven("net.fabricmc:fabric-loader:0.15.11", "https://maven.fabricmc.net/").unwrap();
    assert_eq!(lib.name, "net.fabricmc:fabric-loader:0.15.11");
    assert_eq!(
        lib.downloads.artifact.path,
        "net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar"
    );
    assert_eq!(
        lib.downloads.artifact.url,
        "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar"
    );
}

#[test]
fn test_profile_url_order() {
    let mojang = ManifestUrls::new(Mirror::Mojang);
    assert_eq!(
        mojang.fabric_profile("0.16.14", "1.20.1"),
        "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.16.14/profile/json"
    );
    assert_eq!(
        mojang.quilt_profile("0.24.0", "1.20.1"),
        "https://meta.quiltmc.org/v3/versions/loader/1.20.1/0.24.0/profile/json"
    );
}

#[test]
fn test_loader_versions_forge_strips_mc_prefix() {
    let data = ManifestData {
        minecraft: MinecraftVersionsList {
            latest: LatestMinecraftVersion {
                release: String::new(),
                snapshot: String::new(),
            },
            versions: vec![],
        },
        forge: HashMap::from([
            (
                "1.20.1".to_string(),
                vec!["1.20.1-47.2.0".to_string(), "1.20.1-47.2.1".to_string()],
            ),
        ]),
        fabric: FabricVersionsList { loader: vec![] },
        quilt: QuiltVersionsList { loader: vec![] },
        neoforge: MavenMetadata {
            versioning: MavenVersioning {
                latest: None,
                release: None,
                versions: MavenVersions { version: vec![] },
                last_updated: None,
            },
        },
        liteloader: vec![],
    };

    let versions = loader_versions_from_data(ModLoader::Forge, "1.20.1", &data);
    assert_eq!(versions, vec!["47.2.0", "47.2.1"]);
}

#[test]
fn test_apply_mirror_to_json() {
    let urls = ManifestUrls::new(Mirror::BMCLAPI);
    let profile: serde_json::Value = serde_json::from_str(
        r#"{
            "id": "quilt-loader-0.24.0-1.20.1",
            "libraries": [
                {"name": "net.fabricmc:intermediary:1.20.1", "url": "https://maven.fabricmc.net/"},
                {"name": "org.quiltmc:quilt-loader:0.24.0", "url": "https://maven.quiltmc.org/repository/release/"}
            ]
        }"#,
    )
    .unwrap();

    let mirrored = apply_mirror_to_json(profile, Mirror::BMCLAPI, &urls);
    let libs = mirrored["libraries"].as_array().unwrap();
    assert_eq!(libs[0]["url"], "https://bmclapi2.bangbang93.com/maven/");
    assert_eq!(libs[1]["url"], "https://maven.quiltmc.org/repository/release/");
}

#[test]
fn test_quilt_profile_url_fallback() {
    let bmclapi = ManifestUrls::new(Mirror::BMCLAPI);
    assert_eq!(
        bmclapi.quilt_profile_url("0.24.0", "1.20.1", Mirror::BMCLAPI),
        "https://meta.quiltmc.org/v3/versions/loader/1.20.1/0.24.0/profile/json"
    );

    let mojang = ManifestUrls::new(Mirror::Mojang);
    assert_eq!(
        mojang.quilt_profile_url("0.24.0", "1.20.1", Mirror::Mojang),
        "https://meta.quiltmc.org/v3/versions/loader/1.20.1/0.24.0/profile/json"
    );
}

#[test]
fn test_generated_version_json_serialization() {
    let json = GeneratedVersionJson {
        id: "1.12.2-LiteLoader1.12.2".to_string(),
        inherits_from: Some("1.12.2".to_string()),
        time: Some("2024-01-01T00:00:00Z".to_string()),
        release_time: Some("2024-01-01T00:00:00Z".to_string()),
        version_type: "release".to_string(),
        main_class: "net.minecraft.launchwrapper.Launch".to_string(),
        minecraft_arguments: Some(
            "--username x --tweakClass com.mumfrey.liteloader.launch.LiteLoaderTweaker".to_string(),
        ),
        libraries: vec![GeneratedLibrary::direct(
            "com.mumfrey:liteloader:1.12.2",
            "com/mumfrey/liteloader/1.12.2/liteloader-1.12.2.jar",
            "https://dl.liteloader.com/versions/1.12.2/liteloader-1.12.2.jar",
        )],
    };

    let s = json.to_pretty_json().unwrap();
    assert!(s.contains("\"inheritsFrom\": \"1.12.2\""));
    assert!(s.contains("\"mainClass\": \"net.minecraft.launchwrapper.Launch\""));
    assert!(s.contains("\"minecraftArguments\""));
    assert!(s.contains("com.mumfrey:liteloader:1.12.2"));
    assert!(s.contains("tweakClass"));
}
