//! LiteLoader loader installation (Method B: manual unpack).
//!
//! LiteLoader depends on LaunchWrapper and injects itself into the vanilla launch flow via
//! `--tweakClass`. MDT downloads the core JAR and its dependencies, then synthesizes the version
//! JSON — no Java required.

use crate::error::MdtError;
use crate::modloader::ModLoader;
use crate::urls::ManifestUrls;

use super::version_json::{download_library, read_vanilla_launch_args, GeneratedLibrary, GeneratedVersionJson};
use super::{
    download_to_file, libraries_dir, library_path_from_name, loader_version_id, target_version_dir,
    InstallMethod, InstallRequest, InstalledVersion,
};

pub async fn install(
    http: &reqwest::Client,
    req: InstallRequest,
) -> Result<InstalledVersion, MdtError> {
    let urls = ManifestUrls::try_new(&req.mirror)?;
    let id = loader_version_id(&req.mc_version, ModLoader::Liteloader, &req.loader_version);

    // 1. Download the LiteLoader core JAR (non-standard repository, dedicated download endpoint).
    let liteloader_name = format!("com.mumfrey:liteloader:{}", req.loader_version);
    let liteloader_path = library_path_from_name(&liteloader_name)?;
    let liteloader_dest = libraries_dir(&req.root_dir).join(&liteloader_path);
    let liteloader_url = format!(
        "{}/{}/liteloader-{}.jar",
        urls.liteloader_base, req.mc_version, req.loader_version
    );
    let real_url = urls.apply_mirror(&liteloader_url, &req.mirror);
    download_to_file(http, &real_url, &liteloader_dest).await?;

    // 2. Download LaunchWrapper and ASM (required at runtime).
    download_library(
        http,
        "net.minecraft:launchwrapper:1.12",
        "https://libraries.minecraft.net/",
        &req.root_dir,
        &req.mirror,
        &urls,
    )
    .await?;
    download_library(
        http,
        "org.ow2.asm:asm-all:5.2",
        "https://libraries.minecraft.net/",
        &req.root_dir,
        &req.mirror,
        &urls,
    )
    .await?;

    // 3. Read the vanilla launch args and append the tweak class.
    let (vanilla_args, _) = read_vanilla_launch_args(&req.root_dir, &req.mc_version).await?;
    let mut args = vanilla_args.unwrap_or_else(default_launch_args);
    args.push_str(" --tweakClass com.mumfrey.liteloader.launch.LiteLoaderTweaker");

    // 4. Synthesize the version JSON.
    let json = GeneratedVersionJson {
        id: id.clone(),
        inherits_from: Some(req.mc_version.clone()),
        time: None,
        release_time: None,
        version_type: "release".to_string(),
        main_class: "net.minecraft.launchwrapper.Launch".to_string(),
        minecraft_arguments: Some(args),
        libraries: vec![
            GeneratedLibrary::direct(&liteloader_name, &liteloader_path, &real_url),
            GeneratedLibrary::maven("net.minecraft:launchwrapper:1.12", "https://libraries.minecraft.net/")?,
            GeneratedLibrary::maven("org.ow2.asm:asm-all:5.2", "https://libraries.minecraft.net/")?,
        ],
    };

    let vdir = target_version_dir(&req, &id);
    tokio::fs::create_dir_all(&vdir).await?;
    let json_path = vdir.join(format!("{id}.json"));
    tokio::fs::write(&json_path, json.to_pretty_json()?).await?;

    Ok(InstalledVersion {
        id,
        method: InstallMethod::ManualUnpack,
        version_json: json_path,
        jar: None,
    })
}

fn default_launch_args() -> String {
    "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} \
     --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} \
     --accessToken ${auth_access_token} --userType ${user_type} --versionType ${version_type}"
        .to_string()
}
