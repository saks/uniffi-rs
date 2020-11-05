/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The `cargo uniffi check` command.
//!
//! This command is intended as a light-weight consistency check on the target crate,
//! to make sure it's in a state that can be sensibly built for uniffi. It looks for
//! issues such as:
//!
//!    * missing or incompatible runtime dependencies for uniffi
//!    * incorrect cdylib target configuration
//!
//! Naturally it can't catch everything that might go wrong, but hopefully it will
//! help quickly surface the most common sources of uniffi build errors.

use anyhow::{bail, Result};

use crate::TargetCrate;

/// Add a `clap` argument matcher for the `check` subcommand.
///
pub(crate) fn add_subcommand_matcher<'a, 'b>(matcher: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    matcher.subcommand(
        clap::SubCommand::with_name("check")
            .about("Check that the crate is set up correctly for use with uniffi"),
    )
}

/// Execute the `check` subcommand.
///
//  (TODO: this could probably be split up some!)
//
pub(crate) fn execute_command(
    target: TargetCrate,
    _subargs: Option<&clap::ArgMatches>,
) -> Result<()> {
    // The crate must depend directly on the `uniffi` runtime crate.
    let package = target.root_package()?;
    let uniffi_deps: Vec<&cargo_metadata::Dependency> = package
        .dependencies
        .iter()
        .filter(|p| p.name == "uniffi")
        .collect();
    if uniffi_deps.is_empty() {
        bail!("The crate doesn't depend on the `uniffi` runtime. Please add `uniffi` as a dependency.");
    }
    // The specific resolved versionf of `uniffi` must be compatible with this tool.
    // We can't check this based on the `Dependency` found above because that may specify a version range,
    // we need to look at the actual packages found in the build.
    let uniffi_pkgs: Vec<&cargo_metadata::Package> = target
        .cargo_metadata
        .packages
        .iter()
        .filter(|p| p.name == "uniffi")
        .collect();
    if uniffi_pkgs.is_empty() {
        bail!("The crate doesn't depend on the `uniffi` runtime. Please add `uniffi` as a dependency.");
    }
    if uniffi_pkgs.len() > 1 {
        bail!("The crate depends on multiple versions of `uniffi`. Please rectify the problem and try again.");
    }
    let crate_uniffi_version = uniffi_pkgs[0].version.to_string();
    let our_uniffi_version = crate::UNIFFI_VERSION;
    // XXX: Because we're still < 1.0.0, we compare the entire version string.
    // Once we ship v1, we should compare only the MAJOR component.
    if crate_uniffi_version != our_uniffi_version {
        bail!("The crate depends on a different version of `uniffi` than the `cargo uniffi` command, \
            so bindings generation probably won't work correctly. Please align the versions used \
            by the crate (currently {}) and by this command (currently {}) and try again.",
            crate_uniffi_version,
            our_uniffi_version,
        );
    }
    // The crate must build a single `cdylib` through which to expose its FFI.
    // Trying to locate it will error out for us.
    target.cdylib_target()?;
    // Alrighty, everything seems to be in order here!
    println!("Crate `{}` looks good to me!", package.name);
    Ok(())
}
