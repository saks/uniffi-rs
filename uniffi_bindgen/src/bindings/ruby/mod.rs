/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use std::process::Command;

use crate::interface::{apply_exclusions, rename};
use crate::{bindings::GenerateOptions, BindgenLoader, Component, ComponentInterface};
use anyhow::{bail, Context, Result};
use fs_err as fs;

mod gen_ruby;
#[cfg(feature = "bindgen-tests")]
pub mod test;
use gen_ruby::{Config, RubyWrapper};

pub fn generate(loader: &BindgenLoader, options: GenerateOptions) -> Result<()> {
    let metadata = loader.load_metadata(&options.source)?;
    if let Some(crate_filter) = &options.crate_filter {
        if !metadata.contains_key(crate_filter) {
            bail!("No UniFFI metadata found for crate {crate_filter}");
        }
    }
    let cis = loader.load_cis(&options.source, metadata)?;
    let cdylib = loader.library_name(&options.source).map(|l| l.to_string());
    let mut components =
        loader.load_components(cis, |ci, toml| parse_config(ci, toml, cdylib.clone()))?;
    apply_renames(&mut components);
    for c in components.iter_mut() {
        c.ci.derive_ffi_funcs()?;
    }
    for Component { ci, config, .. } in components {
        if let Some(crate_filter) = &options.crate_filter {
            if ci.crate_name() != crate_filter {
                continue;
            }
        }
        let rb_file = options.out_dir.join(format!("{}.rb", ci.namespace()));
        fs::write(&rb_file, generate_ruby_bindings(&config, &ci)?)?;

        if options.format {
            if let Err(e) = Command::new("rubocop").arg("-A").arg(&rb_file).output() {
                println!(
                    "Warning: Unable to auto-format {} using rubocop: {e:?}",
                    rb_file.file_name().unwrap(),
                )
            }
        }
    }
    Ok(())
}

// Generate ruby bindings for the given ComponentInterface, as a string.
pub fn generate_ruby_bindings(config: &Config, ci: &ComponentInterface) -> Result<String> {
    use askama::Template;
    RubyWrapper::new(config.clone(), ci)
        .render()
        .context("failed to render ruby bindings")
}

fn parse_config(
    ci: &ComponentInterface,
    root_toml: toml::Value,
    cdylib: Option<String>,
) -> Result<Config> {
    let mut config: Config = match root_toml.get("bindings").and_then(|b| b.get("ruby")) {
        Some(v) => v.clone().try_into()?,
        None => Default::default(),
    };
    config.cdylib_name.get_or_insert_with(|| {
        cdylib
            .clone()
            .unwrap_or_else(|| format!("uniffi_{}", ci.namespace()))
    });
    config.normalize_external_package_keys()?;
    Ok(config)
}

fn apply_renames(components: &mut Vec<Component<Config>>) {
    // Remove excluded items, this happens before renaming
    for c in components.iter_mut() {
        apply_exclusions(&mut c.ci, &c.config.exclude);
    }

    let mut module_renames = HashMap::new();
    for c in components.iter() {
        if !c.config.rename.is_empty() {
            let module_path = c.ci.crate_name().to_string();
            module_renames.insert(module_path, c.config.rename.clone());
        }
    }

    if !module_renames.is_empty() {
        for c in &mut *components {
            rename(&mut c.ci, &module_renames);
        }
    }

    populate_external_packages(components);
}

/// Fill each crate's `external_packages` from peer crates.
///
/// Default module name is the peer's namespace converted to UpperCamelCase.
/// User overrides in `[bindings.ruby.external_packages]` win; keys must already
/// be normalized (`my-crate` → `my_crate`) via [`Config::normalize_external_package_keys`].
fn populate_external_packages(components: &mut [Component<Config>]) {
    let packages = HashMap::<String, String>::from_iter(components.iter().map(|c| {
        (
            c.ci.crate_name().to_string(),
            heck::ToUpperCamelCase::to_upper_camel_case(c.ci.namespace()),
        )
    }));
    for c in components.iter_mut() {
        for (ext_crate, ext_package) in &packages {
            if ext_crate != c.ci.crate_name() && !c.config.external_packages.contains_key(ext_crate)
            {
                c.config
                    .external_packages
                    .insert(ext_crate.to_string(), ext_package.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(crate_name: &str, config: Config) -> Component<Config> {
        Component {
            ci: ComponentInterface::new(crate_name),
            config,
        }
    }

    #[test]
    fn hyphenated_user_override_is_not_clobbered_by_auto_pop() {
        let mut consumer_config = Config::default();
        consumer_config
            .external_packages
            .insert("my-crate".into(), "Custom".into());
        consumer_config.normalize_external_package_keys().unwrap();

        let mut components = vec![
            component("consumer", consumer_config),
            component("my_crate", Config::default()),
        ];
        populate_external_packages(&mut components);
        assert_eq!(
            components[0]
                .config
                .external_packages
                .get("my_crate")
                .map(String::as_str),
            Some("Custom")
        );
    }

    #[test]
    fn auto_pop_fills_missing_peer_crate() {
        let mut components = vec![
            component("consumer", Config::default()),
            component("my_crate", Config::default()),
        ];
        populate_external_packages(&mut components);
        assert!(components[0]
            .config
            .external_packages
            .contains_key("my_crate"));
        assert!(!components[0]
            .config
            .external_packages
            .contains_key("consumer"));
    }
}
