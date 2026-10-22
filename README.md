# MDT — Minecraft Download Toolkit

[![Crates.io](https://img.shields.io/crates/v/mdt-minecraft.svg)](https://crates.io/crates/mdt-minecraft)
[![Documentation](https://docs.rs/mdt-minecraft/badge.svg)](https://docs.rs/mdt-minecraft)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Here is how you build a launcher in three lines.**

```rust
use mdt::{MdtClient, Mirror, ModLoader, mdt_install};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MdtClient::builder()
        .mirror(Mirror::BMCLAPI)
        .root_dir("./.minecraft")
        .build()?;

    client.install_vanilla("1.20.1").await?;                                    // vanilla
    mdt_install!(client, "1.20.1", ModLoader::Fabric, "0.15.11").await?;        // Fabric

    Ok(())
}
```

MDT is a low-level, ergonomic **library** for downloading and installing Minecraft versions and
mod loaders. It is the dirt-handling layer a launcher sits on top of: no TUI, no progress bars,
no user input — just a strict, typed API that returns real errors.

## Isolation guarantee

Every loader install lands in its own directory named after the convention
`<mc_version>+<Loader><loader_version>`:

```
.minecraft/
└── versions/
    ├── 1.20.1/                  # vanilla — never touched by a loader install
    │   ├── 1.20.1.json
    │   └── 1.20.1.jar
    └── 1.20.1+Fabric0.15.11/    # isolated loader install
        └── 1.20.1+Fabric0.15.11.json
```

A broken loader can never corrupt the vanilla game. You don't have to write a single line of
code to enforce this — the library does it for you.

## Macros

```rust
use mdt::{version_dir, download_task};

// One expression for the isolated version path (loader as a bare variant name).
let dir = version_dir!(client, "1.16.5", Fabric, "0.20.0");
// → <root>/versions/1.16.5+Fabric0.20.0

// One expression for a download task (size defaults to 0, extract_to to None).
let task = download_task!("https://…/lib.jar", "libraries/lib.jar", "abc123", 42);
```

## Errors

Every fallible operation returns `Result<T, MdtError>`. Match on it directly (the enum is
`#[non_exhaustive]`, so keep a wildcard `_` arm):

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

## Mirrors

| Variant           | Description                                                                      |
|-------------------|----------------------------------------------------------------------------------|
| `Mirror::BMCLAPI` | Community mirror (default). Proxies Mojang, Fabric, Forge, NeoForge, LiteLoader. |
| `Mirror::Mojang`  | Official upstream.                                                               |

Failed requests automatically fall back to the opposite mirror.

## Loaders

| Loader                  | Method                  | Notes                                        |
|-------------------------|-------------------------|----------------------------------------------|
| `ModLoader::Forge`      | A (1.13+) / B (1.12.2-) | Official installer + Java, or manual unpack. |
| `ModLoader::Neoforge`   | A                       | Official installer + Java.                   |
| `ModLoader::Fabric`     | B                       | Profile-based, no Java required.             |
| `ModLoader::Quilt`      | B                       | Profile-based, no Java required.             |
| `ModLoader::Liteloader` | B                       | 1.12.2 and earlier only.                     |

Method **A** requires a local Java runtime (`JAVA_HOME` or `java` on `PATH`).

## Feature flags

The core library is **100% terminal-agnostic**. Terminal UI and CLI parsing are strictly
feature-gated, so `crossterm`, `ratatui`, and `clap` are never compiled unless requested:

```toml
mdt = { version = "0.3", features = ["cli"] }   # CLI + TUI
mdt = { version = "0.3", features = ["tui"] }   # TUI only
mdt = "0.3"                                     # library only (no terminal deps)
```

## License

MIT
