/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Sub-command implementations for `cargo uniffi`.
//!
//! This module contains the various subcommands for `cargo uniffi`, and a bit
//! of glue code for dispatching to them based on command-line arguments.

use anyhow::{bail, Result};

mod bindgen;
mod check;
mod gradle;

use crate::TargetCrate;

/// Add a `clap` argument matcher for each available subcommand.
///
pub(crate) fn add_subcommand_matchers<'a, 'b>(matcher: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    let matcher = bindgen::add_subcommand_matcher(matcher);
    let matcher = check::add_subcommand_matcher(matcher);
    let matcher = gradle::add_subcommand_matcher(matcher);
    matcher
}

/// Execute the `cargo uniffi` command specified by the given command-line arguments.
///
pub(crate) fn execute_command(target: TargetCrate, args: clap::ArgMatches) -> Result<()> {
    match args.subcommand() {
        ("bindgen", subargs) => bindgen::execute_command(target, subargs)?,
        ("check", subargs) => check::execute_command(target, subargs)?,
        ("gradle", subargs) => gradle::execute_command(target, subargs)?,
        _ => {
            // In the future we could do some extensibility cleverness here and
            // look for `cargo-uniffi-${command}` in your path, like cargo does.
            bail!("No command specified; try `--help` for some help.")
        }
    }
    Ok(())
}
