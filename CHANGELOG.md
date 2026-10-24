# Changelog

## [0.5.0] | Modloader (Un)Install Mechanism Update

- **Uninstall** — `MdtClient::uninstall_modloader` reverts a loader back to pure vanilla using a
  SHA-1 manifest comparison. It pre-loads the manifest table into memory (no N+1 network queries),
  scans on disk with a multithreaded bucket-sorted scanner, and applies MCR markers
  (`[Hash].pinned` modified, `[Hash].pinned.add` created, `[Filename].pinned.remove` removed).
  The `mods/` folder is preserved. Re-running it is safe.
- **1 Pub Function(s) Breaking** — `MdtClient::install_modloader` now takes a vanilla version directory (`&Path`)
  plus a `copy_vanilla: bool` instead of a `&str` version. The Minecraft version is read from the
  directory's `version.json` `id` field and fails fast with `MdtError::InvalidManifest` if that
  JSON is missing or malformed.
- **Copy-on-write** — with `copy_vanilla == true` the vanilla directory is copied into the
  isolated `versions/<mc>+<Loader><ver>` (or the official installer's `-forge-`/`-neoforge-`)
  directory first; with `false` the loader is installed in place, mutating the vanilla directory.
- **New error** — `MdtError::UninstallFailed`.
- **Macro Name & Mechanism Breaking** — the `mdt_install!` macro is renamed to `full_install!`, and it now
  installs vanilla and the loader in one shot (with a five-argument form for the copy-on-write
  flag). It expands to the new `MdtClient::full_install`.

## [0.4.0] | custom mirror providers

Mirror endpoints and URL-rewrite rules moved out of the code into `providers.toml`. They're
compiled into static lookup tables at build time by the `mdt-macros::providers!` proc macro
(MDT's take on `rust-i18n`'s `i18n!`), so nothing is parsed at runtime.

- **Custom mirrors** — `Mirror::Custom("name")` picks a `[name]` section from `providers.toml`,
  with its own endpoints and ordered `mirror` / `maven` rewrite rules (plus a `maven_exclude` list).
- **Fallible lookup** — `ManifestUrls::new` split into `try_new` (returns `MdtError::UnknownProvider`)
  and `new` (panics).
- **New error** — `MdtError::UnknownProvider`.
- **Breaking** — `Mirror` is now `Clone` (not `Copy`), and mirrors are passed by reference (`&Mirror`).
- **Docs** — `docs/mirror_toml.md` and `docs/mirror_uri.md`.
- **Workspace** — `mdt-macros` is now a separate member crate.

## [0.3.1] | Dependencies Update

Added the README badge and bumped Tokio.

## [0.3.0] | First Released

First release of MDT as a library.
