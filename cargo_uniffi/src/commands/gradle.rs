/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The `cargo uniffi bindgen` command.
//!
//! This command is used to generate foreign-language bindings for the specified
//! crate, based on its UDL and uniffi configuration. It's currently a thin wrapper
//! around the existing `uniffi_bindgen` crate, but if we like this `cargo uniffi`
//! approach them we could refactor that some.

use std::io::Write;

use anyhow::{Context, Result, bail};
use askama::Template;

use crate::TargetCrate;

/// Add a `clap` argument matcher for the `bindgen` subcommand.
///
pub(crate) fn add_subcommand_matcher<'a, 'b>(matcher: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    matcher.subcommand(
        clap::SubCommand::with_name("gradle")
            .about("Generate and run commands in a gradle project for the crate")
            .arg(
                clap::Arg::with_name("gradle_args")
                    .multiple(true)
                    .help("Additional arguments to pass to invocation of gradle"),
            ),
    )
}

/// Execute the `gradle` subcommand.
///
pub(crate) fn execute_command(
    target: TargetCrate,
    subargs: Option<&clap::ArgMatches>,
) -> Result<()> {
    let gradle_args: Vec<String> = match subargs {
        None => vec![],
        Some(args) => args
            .values_of("gradle_args")
            .into_iter()
            .flatten()
            .map(|s| s.into())
            .collect(),
    };
    let project_dir = ensure_gradle_project(target)?;
    execute_gradle(&project_dir, &gradle_args)?;
    Ok(())
}

/// Template for generating the `build.gradle` file for a uniffi component.
///
#[derive(Template)]
#[template(escape = "none", path = "build.gradle")]
struct GradleBuildFile<'a> {
    target: &'a TargetCrate,
}
impl<'a> GradleBuildFile<'a> {
    pub fn generate(target: &'a TargetCrate, path: &std::path::Path) -> Result<()> {
        let template = Self { target };
        let mut f = std::fs::File::create(&path)?;
        write!(
            f,
            "{}",
            template
                .render()
                .context("Failed to render build.gradle file")?
        )?;
        Ok(())
    }
}

/// Generate a temporary gradle project directory for a uniffi component.
///
fn ensure_gradle_project(target: TargetCrate) -> Result<std::path::PathBuf> {
    // TODO: see if there's any more nuanced way to get a cargo "cache" directory of some sort.
    // Needs to look at environment variables etc.
    let project_dir = target.cargo_metadata.target_directory.join("uniffi");
    std::fs::create_dir_all(&project_dir)?;
    let build_file = project_dir.join("build.gradle");
    GradleBuildFile::generate(&target, &build_file)?;
    Ok(project_dir)
}

/// Execute gradle in the given directory, with specified args.
///
//
//  TODO: need to figure out the details of this, currently kind of assumes that
//  gradle is available and doesn't have good control of the versions etc.
//  I suppose we could template-generate the gradle-wrapper stuff and let it works
//  its magic.
//
fn execute_gradle(project_dir: &std::path::Path, args: &[String]) -> Result<()> {
    let status = std::process::Command::new("gradle")
        .args(args)
        .spawn()
        .context("Failed to spawn `gradle`")?
        .wait()
        .context("Failed to wait for `gradle`")?;
    if !status.success() {
        // TODO: maybe we should propagate this error code, even exit with it?
        bail!("running `gradle` failed")
    }
    Ok(())
}
