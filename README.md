# MDT — Minecraft Download Toolkit

Download and install Minecraft versions and mod loaders from Rust.

[![Crates.io](https://img.shields.io/crates/v/mdt-minecraft.svg)](https://crates.io/crates/mdt-minecraft)
[![docs.rs](https://docs.rs/mdt-minecraft/badge.svg)](https://docs.rs/mdt-minecraft)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

MDT is a small library that fills a `.minecraft` folder for you — vanilla versions, mod
loaders, libraries, assets. It's the plumbing a launcher sits on top of: no UI, no progress
bars, no prompts. You call a function, you get a `Result`.

## Features

- Vanilla Minecraft, plus Forge, Fabric, Quilt, NeoForge, and LiteLoader.
- Isolated installs — each loader lands in its own `versions/<mc>+<Loader><ver>` folder, so a
  broken loader never touches your vanilla game.
- SHA-1 verified downloads with automatic mirror fallback when a source is slow or blocked.
- Custom mirrors via a `providers.toml` you can edit (compiled in at build time).
- No terminal dependencies in the core; `cli` and `tui` are opt-in features.

## Install

```toml
[dependencies]
mdt-minecraft = "0.4"
```

## Quick start

```rust
use mdt::{MdtClient, Mirror, ModLoader, mdt_install};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MdtClient::builder()
        .mirror(Mirror::BMCLAPI)
        .root_dir("./.minecraft")
        .build()?;

    client.install_vanilla("1.20.1").await?;                                   // vanilla
    mdt_install!(client, "1.20.1", ModLoader::Fabric, "0.15.11").await?;       // Fabric

    Ok(())
}
```

## Isolation

Each loader install goes into its own directory, separate from the vanilla version:

```
.minecraft/
└── versions/
    ├── 1.20.1/                  # vanilla — never touched by a loader install
    └── 1.20.1+Fabric0.15.11/    # isolated Fabric install
```

You don't have to enforce this yourself — the library does it.

## Mirrors

| Variant                  | What it is                                                                       |
|--------------------------|----------------------------------------------------------------------------------|
| `Mirror::BMCLAPI`        | Community mirror (default). Proxies Mojang, Fabric, Forge, NeoForge, LiteLoader. |
| `Mirror::Mojang`         | Official servers.                                                                |
| `Mirror::Custom("name")` | Your own provider, declared as `[name]` in `providers.toml`.                     |

Failed requests fall back to the opposite mirror automatically (custom mirrors fall back to
`BMCLAPI`). Endpoints and rewrite rules live in [`providers.toml`](providers.toml) — see
[docs/mirror_toml.md](docs/mirror_toml.md) and [docs/mirror_uri.md](docs/mirror_uri.md).

## Loaders

| Loader                  | Method                  | Notes                                        |
|-------------------------|-------------------------|----------------------------------------------|
| `ModLoader::Forge`      | A (1.13+) / B (1.12.2-) | Official installer + Java, or manual unpack. |
| `ModLoader::Neoforge`   | A                       | Official installer + Java.                   |
| `ModLoader::Fabric`     | B                       | No Java needed.                              |
| `ModLoader::Quilt`      | B                       | No Java needed.                              |
| `ModLoader::Liteloader` | B                       | 1.12.2 and earlier.                          |

Method A needs a local Java runtime (`JAVA_HOME` or `java` on `PATH`).

## Errors

Every fallible call returns `Result<T, MdtError>`, so you can match on exactly what went wrong:

```rust
match client.install_modloader("1.20.1", ModLoader::Forge, "47.2.0").await {
    Ok(v)   => println!("installed {} via {}", v.id, v.method),
    Err(e)  => match e {
        mdt::MdtError::JavaNotFound          => eprintln!("install Java first"),
        mdt::MdtError::ChecksumMismatch{..}  => eprintln!("corrupt download"),
        other                                => eprintln!("{other}"),
    },
}
```

## Macros

A few one-liners for common things:

```rust
use mdt::{version_dir, download_task};

let dir  = version_dir!(client, "1.16.5", Fabric, "0.20.0");  // <root>/versions/1.16.5+Fabric0.20.0
let task = download_task!("https://…/lib.jar", "libraries/lib.jar", "abc123", 42);
```

## Feature flags

The core library has no terminal dependencies by default:

```toml
mdt-minecraft = "0.4"                                              # library only
mdt-minecraft = { version = "0.4", features = ["tui"] }            # + TUI
mdt-minecraft = { version = "0.4", features = ["cli"] }            # + CLI and TUI
```

## Docs

- [docs.rs/mdt-minecraft](https://docs.rs/mdt-minecraft)
- [Mirror TOML format](docs/mirror_toml.md)
- [Mirror URI spec](docs/mirror_uri.md)

## Changelog

See [CHANGELOG.md](CHANGELOG.md) and the [releases](https://github.com/Lavaver/MDT/releases) page.

## License

[MIT](LICENSE)
