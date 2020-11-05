/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::{anyhow, bail, Context, Result};

mod commands;

pub(crate) const UNIFFI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build a `clap` argument matcher for the `cargo uniffi` command.
///
pub fn build_arg_matcher() -> clap::App<'static, 'static> {
    let matcher = clap::App::new("cargo-uniffi")
        .about("Multi-language bindings generator for Rust")
        .arg(
            clap::Arg::with_name("manifest_path")
                .long("manifest-path")
                .value_name("PATH")
                .help("Path to Cargo.toml")
                .takes_value(true),
        );
    commands::add_subcommand_matchers(matcher)
}

/// Execute the `cargo uniffi` command specified by the given command-line arguments.
///
pub fn execute_command(args: clap::ArgMatches) -> Result<()> {
    let target = TargetCrate::from_args(&args)?;
    commands::execute_command(target, args)
}

/// Metadata for the target crate on which `uniffi` should act.
///
/// The `cargo uniffi` command is designed to work on a crate in a similar way
/// to builtin commands like `cargo build`, and this struct encapsulates all
/// the info about the target crate that we need to know in order to operate
/// on it.
///
//  TODO: it might be worthwhile to locate the UDL file, config file etc once
//  on initialization and cache them, rather than searching for them on demand.
//
pub(crate) struct TargetCrate {
    manifest_path: std::path::PathBuf,
    cargo_metadata: cargo_metadata::Metadata,
}

impl TargetCrate {
    /// Determine target crate metadata from command-line arguments.
    ///
    pub fn from_args(args: &clap::ArgMatches) -> Result<Self> {
        let manifest_path = args.value_of_os("manifest_path");
        let mut metadata_cmd = cargo_metadata::MetadataCommand::new();
        if let Some(path) = manifest_path {
            metadata_cmd.manifest_path(path);
        }
        let metadata = metadata_cmd
            .exec()
            .with_context(|| format!("Failed to read crate metadata"))?;
        let package = match metadata.root_package() {
            None => bail!("Could not determine root package metadata. Please specify an individual crate, not a workspace."),
            Some(pkg) => pkg,
        };
        Ok(TargetCrate {
            manifest_path: package.manifest_path.clone(),
            cargo_metadata: metadata,
        })
    }

    /// Get the metadata for the root package of the crate.
    ///
    /// This is mostly a convenience wrapper to throw a sensible error
    /// if no root package can be found.
    pub fn root_package(&self) -> Result<&cargo_metadata::Package> {
        match self.cargo_metadata.root_package() {
            None => bail!("Could not determine root package metadata. Please specify an individual crate, not a workspace."),
            Some(pkg) => Ok(pkg),
        }
    }

    /// Get the path to the crate's UDL interface file.
    ///
    pub fn udl_file(&self) -> Result<std::path::PathBuf> {
        if let Some(src_dir) = self.cdylib_target()?.src_path.parent() {
            // Lightly hacky: look for a single `*.udl` file in the source directory.
            // XXX I think ideally we'd read this from a config file or something.
            let mut udl_files = src_dir
                .read_dir()
                .context("Failed to list target source directory")?
                .into_iter()
                .map(|entry| entry.context("Failed to list target source directory"))
                .filter_map(|entry| {
                    entry
                        .map(|e| {
                            if e.path().extension() == Some(std::ffi::OsStr::new("udl")) {
                                Some(e.path())
                            } else {
                                None
                            }
                        })
                        .transpose()
                })
                .collect::<Result<Vec<_>>>()?;
            if udl_files.is_empty() {
                bail!("Could not find a `.udl` file in target source directory")
            }
            if udl_files.len() > 1 {
                bail!("Found multiple `.udl` files in target source directory")
            }
            udl_files
                .pop()
                .ok_or_else(|| anyhow!("Could not find a `.udl` file in target source directory"))
        } else {
            bail!("Target source file has not parent directory")
        }
    }

    /// Get the path to the crate's config file.
    ///
    /// We currently use a file named `uniffi.toml` in the crate root to control
    /// some aspects of bindings generation, and this method will locate it.
    ///
    /// I'd like to explore a few different approaches to this, such as using custom
    /// sections in `Cargo.toml` rather than a separate file. If we like this `cargo uniffi`
    /// approach, we could also consider exposing the `uniffi_bindgen::Config` struct parsing
    /// here more directly.
    ///
    pub fn config_file(&self) -> Result<Option<std::path::PathBuf>> {
        let config_file = self
            .manifest_path
            .parent()
            .ok_or_else(|| anyhow!("Cargo.toml has no parent directory"))?
            .join("uniffi.toml");
        if config_file.is_file() {
            Ok(Some(config_file))
        } else {
            Ok(None)
        }
    }

    /// Get metadata about the cdylib target for this crate.
    ///
    pub fn cdylib_target(&self) -> Result<&cargo_metadata::Target> {
        let cdylibs: Vec<_> = self
            .root_package()?
            .targets
            .iter()
            .filter(|t| t.kind.iter().any(|kind| kind == "lib"))
            .collect();
        if cdylibs.is_empty() {
            bail!("The crate doesn't build a dynamic library. Please specify `crate-type = [\"cdylib\"]` in your crate manifest.");
        }
        if cdylibs.len() > 1 {
            bail!("The crate builds multiple dynamic libraries. Please adjust your crate manifest to have a single `cdylib` target.");
        }
        Ok(cdylibs[0])
    }
}
