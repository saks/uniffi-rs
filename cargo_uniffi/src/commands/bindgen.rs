/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The `cargo uniffi bindgen` command.
//!
//! This command is used to generate foreign-language bindings for the specified
//! crate, based on its UDL and uniffi configuration. It's currently a thin wrapper
//! around the existing `uniffi_bindgen` crate, but if we like this `cargo uniffi`
//! approach them we could refactor that some.

use anyhow::Result;

use crate::TargetCrate;

const POSSIBLE_LANGUAGES: &[&str] = &["kotlin", "python", "swift", "gecko_js"];

/// Add a `clap` argument matcher for the `bindgen` subcommand.
///
pub(crate) fn add_subcommand_matcher<'a, 'b>(matcher: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    matcher.subcommand(
clap::SubCommand::with_name("bindgen")
        .about("Generate foreign language bindings")
        .arg(
            clap::Arg::with_name("language")
                .required(true)
                .takes_value(true)
                .long("--language")
                .short("-l")
                .multiple(true)
                .number_of_values(1)
                .possible_values(&POSSIBLE_LANGUAGES)
                .help("Foreign language(s) for which to generate bindings"),
        )
        .arg(
            clap::Arg::with_name("out_dir")
                .long("--out-dir")
                .short("-o")
                .takes_value(true)
                .help("Directory in which to write generated files. Default is same folder as .udl file."),
        )
        .arg(
            clap::Arg::with_name("no_format")
                .long("--no-format")
                .help("Do not try to format the generated bindings"),
        )
    )
}

/// Execute the `bindgen` subcommand.
///
pub(crate) fn execute_command(
    target: TargetCrate,
    subargs: Option<&clap::ArgMatches>,
) -> Result<()> {
    let subargs = subargs.expect("Should always have subargs, since one is required");
    uniffi_bindgen::generate_bindings(
        target.udl_file()?,
        target.config_file()?,
        subargs.values_of("language").unwrap().collect(), // Required
        subargs
            .value_of_os("out_dir")
            .map(|p| std::path::Path::new(p).to_path_buf()),
        !subargs.is_present("no_format"),
    )?;
    Ok(())
}
